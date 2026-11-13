//! Boolean guard facts shared across unchanged CFG edges. This contains only
//! the compiler's pure dataflow state; it grants no runtime permissions.
use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Expression {
    Copy(u32),
    Not(u32),
    And(u32, u32),
    Or(u32, u32),
    Project { source: u32, path: Vec<(u8, u32)> },
}
impl Expression {
    fn reads(&self, value: u32) -> bool {
        match self {
            Self::Copy(a) | Self::Not(a) => *a == value,
            Self::And(a, b) | Self::Or(a, b) => *a == value || *b == value,
            Self::Project { source, .. } => *source == value,
        }
    }
}
// Plan value IDs are checked, dense arena indices. Two bits encode an
// unknown/false/true fact; intersection is a wordwise AND, including conflicts.
// Storage remains shared until an actual fact changes.
#[derive(Clone, Default, Debug, Eq, PartialEq)]
struct Known {
    words: Arc<Vec<u64>>,
    len: usize,
}
impl Known {
    fn get(&self, value: &u32) -> Option<&bool> {
        let shift = (value % 32) * 2;
        match (self.words.get((*value / 32) as usize).copied().unwrap_or(0) >> shift) & 3 {
            1 => Some(&false),
            2 => Some(&true),
            _ => None,
        }
    }
    fn contains_key(&self, value: &u32) -> bool {
        self.get(value).is_some()
    }
    fn len(&self) -> usize {
        self.len
    }
    fn is_empty(&self) -> bool {
        self.len == 0
    }
    fn insert(&mut self, value: u32, truth: bool) {
        let previous = self.get(&value).copied();
        if previous == Some(truth) {
            return;
        }
        let index = (value / 32) as usize;
        let shift = (value % 32) * 2;
        let words = Arc::make_mut(&mut self.words);
        if words.len() <= index {
            words.resize(index + 1, 0);
        }
        words[index] = (words[index] & !(3u64 << shift)) | ((if truth { 2 } else { 1 }) << shift);
        if previous.is_none() {
            self.len += 1;
        }
    }
    fn remove(&mut self, value: &u32) {
        if self.contains_key(value) {
            Arc::make_mut(&mut self.words)[(*value / 32) as usize] &= !(3u64 << ((value % 32) * 2));
            self.len -= 1;
        }
    }
    fn intersect(&mut self, incoming: &Self) -> bool {
        if Arc::ptr_eq(&self.words, &incoming.words) {
            return false;
        }
        let retained = self
            .words
            .iter()
            .zip(incoming.words.iter())
            .map(|(a, b)| (a & b).count_ones() as usize)
            .sum::<usize>();
        if retained == self.len {
            if retained == incoming.len {
                *self = incoming.clone();
            }
            return false;
        }
        if retained == incoming.len {
            *self = incoming.clone();
        } else if retained == 0 {
            *self = Self::default();
        } else {
            let words = Arc::make_mut(&mut self.words);
            words.truncate(incoming.words.len());
            for (word, incoming) in words.iter_mut().zip(incoming.words.iter()) {
                *word &= incoming;
            }
            self.len = retained;
        }
        true
    }
    #[cfg(test)]
    fn to_map(&self) -> BTreeMap<u32, bool> {
        let mut result = BTreeMap::new();
        for (index, &word) in self.words.iter().enumerate() {
            for lane in 0..32 {
                match (word >> (lane * 2)) & 3 {
                    1 => {
                        result.insert((index * 32 + lane) as u32, false);
                    }
                    2 => {
                        result.insert((index * 32 + lane) as u32, true);
                    }
                    0 => {}
                    _ => panic!("conflicting packed Boolean encoding"),
                }
            }
        }
        result
    }
}

// Derived solely from one immutable relation set. Truths remain in each State;
// this shared index contains no conclusions or runtime ownership permission.
#[derive(Debug)]
struct RelationIndex {
    rows: Vec<(u32, Expression)>,
    related: Vec<u32>,
    projection_group: Vec<usize>,
    groups: Vec<Vec<u32>>,
}
impl RelationIndex {
    fn new(expressions: &BTreeMap<u32, Expression>) -> Self {
        let rows = expressions
            .iter()
            .map(|(&out, expression)| (out, expression.clone()))
            .collect::<Vec<_>>();
        let mut related = Vec::new();
        let mut projection_group = vec![usize::MAX; rows.len()];
        let mut groups = Vec::<Vec<u32>>::new();
        let mut group_ids = BTreeMap::new();
        for (position, (out, expression)) in rows.iter().enumerate() {
            related.push(*out);
            match expression {
                Expression::Copy(a) | Expression::Not(a) => related.push(*a),
                Expression::And(a, b) | Expression::Or(a, b) => related.extend([*a, *b]),
                Expression::Project { source, path } => {
                    related.push(*source);
                    let group = *group_ids
                        .entry((*source, path.as_slice()))
                        .or_insert_with(|| {
                            groups.push(Vec::new());
                            groups.len() - 1
                        });
                    projection_group[position] = group;
                    groups[group].push(*out);
                }
            }
        }
        drop(group_ids);
        related.sort_unstable();
        related.dedup();
        Self {
            rows,
            related,
            projection_group,
            groups,
        }
    }
}

#[derive(Clone, Default, Debug)]
pub(crate) struct State {
    // Most CFG edges preserve both maps. A state forks its storage only when
    // a fact or relation actually changes.
    known: Known,
    expressions: Arc<BTreeMap<u32, Expression>>,
    index: Arc<OnceLock<RelationIndex>>,
    closed: bool,
}
impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.known == other.known
            && self.expressions == other.expressions
            && self.closed == other.closed
    }
}
impl Eq for State {}
impl State {
    pub(super) fn truth(&self, value: u32) -> Option<bool> {
        self.known.get(&value).copied()
    }
    pub fn is_empty(&self) -> bool {
        self.known.is_empty()
    }
    pub(super) fn assume(&mut self, value: u32, truth: bool) {
        if self.known.get(&value) != Some(&truth) {
            // A fact outside every equation cannot enable another inference.
            // Keep any already pending closure pending, even for such a fact.
            let requires_closure = !self.closed
                || self
                    .index
                    .get_or_init(|| RelationIndex::new(&self.expressions))
                    .related
                    .binary_search(&value)
                    .is_ok();
            self.known.insert(value, truth);
            self.closed &= !requires_closure;
        }
    }
    pub fn merge(&mut self, other: &Self) -> bool {
        let known_changed = self.known.intersect(&other.known);
        let expressions_changed = intersect(&mut self.expressions, &other.expressions);
        if expressions_changed {
            self.index = if Arc::ptr_eq(&self.expressions, &other.expressions) {
                other.index.clone()
            } else {
                Arc::default()
            };
        }
        let changed = known_changed || expressions_changed;
        // An intersection can require closure even when neither predecessor
        // changes again, including contradictory syntactic branch paths.
        if changed {
            self.closed = false;
        }
        changed
    }
    pub(super) fn close(&mut self) {
        if self.closed {
            return;
        }
        // Rows and projection peers retain the original ascending-output order,
        // including first-inference priority on contradictory syntactic paths.
        let index = self
            .index
            .get_or_init(|| RelationIndex::new(&self.expressions));
        loop {
            let mut inferred = Vec::new();
            let mut infer = |value, truth| {
                if !self.known.contains_key(&value) {
                    inferred.push((value, truth));
                }
            };
            for (position, (out, e)) in index.rows.iter().enumerate() {
                let out = *out;
                let truth = self.known.get(&out).copied();
                match *e {
                    Expression::Copy(a) | Expression::Not(a) => {
                        let invert = matches!(e, Expression::Not(_));
                        if let Some(t) = truth {
                            infer(a, t ^ invert);
                        }
                        if let Some(&t) = self.known.get(&a) {
                            infer(out, t ^ invert);
                        }
                    }
                    Expression::And(a, b) | Expression::Or(a, b) => {
                        let conjunction = matches!(e, Expression::And(..));
                        let left = self.known.get(&a).copied();
                        let right = self.known.get(&b).copied();
                        if truth == Some(conjunction) {
                            infer(a, conjunction);
                            infer(b, conjunction);
                        } else if truth == Some(!conjunction) {
                            if left == Some(conjunction) {
                                infer(b, !conjunction);
                            }
                            if right == Some(conjunction) {
                                infer(a, !conjunction);
                            }
                        }
                        if left == Some(!conjunction) || right == Some(!conjunction) {
                            infer(out, !conjunction);
                        } else if left == Some(conjunction) && right == Some(conjunction) {
                            infer(out, conjunction);
                        }
                    }
                    Expression::Project { .. } => {
                        if let Some(t) = truth {
                            for &peer in &index.groups[index.projection_group[position]] {
                                infer(peer, t);
                            }
                        }
                    }
                }
            }
            let before = self.known.len();
            for (value, truth) in inferred {
                if !self.known.contains_key(&value) {
                    self.known.insert(value, truth);
                }
            }
            if self.known.len() == before {
                break;
            }
        }
        self.closed = true;
    }
    pub(super) fn define(&mut self, defs: &[u32], expression: Option<&(u32, Expression)>) {
        if defs.is_empty() && expression.is_none() {
            self.close();
            return;
        }
        // Expiration removes each defined fact and every relation that mentions
        // it. Remaining equations keep exactly the same inputs; deleting an
        // equation cannot add a new inference. Preserve an already closed state,
        // while leaving a pending closure pending. New equations below still
        // invalidate closure.
        let mut relations_changed = false;
        for value in defs {
            if self.known.contains_key(value) {
                self.known.remove(value);
            }
        }
        let expires_relation = if let Some(index) = self.index.get() {
            defs.iter()
                .any(|value| index.related.binary_search(value).is_ok())
        } else {
            self.expressions
                .iter()
                .any(|(out, e)| defs.contains(out) || defs.iter().any(|&v| e.reads(v)))
        };
        if expires_relation {
            Arc::make_mut(&mut self.expressions)
                .retain(|out, e| !defs.contains(out) && !defs.iter().any(|&v| e.reads(v)));
            relations_changed = true;
        }
        if let Some((output, e)) = expression {
            // An in-place definition reads the preceding version. It cannot
            // establish an equation about the new input home without a phi.
            if !defs.iter().any(|&v| e.reads(v)) {
                if self.expressions.get(output) != Some(e) {
                    Arc::make_mut(&mut self.expressions).insert(*output, e.clone());
                    relations_changed = true;
                    self.closed = false;
                }
            }
        }
        if relations_changed {
            self.index = Arc::default();
        }
        self.close();
    }
}

// Sorted intersection visits each input entry once. It does not repeatedly
// search a tree for every fact, or copy a map when a predecessor already owns
// the exact intersection.
fn intersect<V: Clone + Eq>(
    current: &mut Arc<BTreeMap<u32, V>>,
    incoming: &Arc<BTreeMap<u32, V>>,
) -> bool {
    if Arc::ptr_eq(current, incoming) {
        return false;
    }
    let mut peers = incoming.iter().peekable();
    let mut removed = Vec::new();
    let mut retained = 0;
    for (&key, value) in current.iter() {
        while peers.peek().is_some_and(|(peer, _)| **peer < key) {
            peers.next();
        }
        if peers
            .peek()
            .is_some_and(|(peer, other)| **peer == key && *other == value)
        {
            retained += 1;
        } else {
            removed.push(key);
        }
    }
    if removed.is_empty() {
        if current.len() == incoming.len() {
            *current = incoming.clone();
        }
        return false;
    }
    if retained == incoming.len() {
        *current = incoming.clone();
    } else if retained == 0 {
        *current = Arc::default();
    } else {
        let mut removed = removed.into_iter().peekable();
        Arc::make_mut(current).retain(|key, _| {
            if removed.peek() == Some(key) {
                removed.next();
                false
            } else {
                true
            }
        });
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Clone, Default, Debug, Eq, PartialEq)]
    struct ReferenceState {
        known: BTreeMap<u32, bool>,
        expressions: BTreeMap<u32, Expression>,
    }
    impl ReferenceState {
        pub fn is_empty(&self) -> bool {
            self.known.is_empty()
        }
        pub fn merge(&mut self, other: &Self) -> bool {
            let before = (self.known.len(), self.expressions.len());
            self.known.retain(|v, t| other.known.get(v) == Some(t));
            self.expressions
                .retain(|v, e| other.expressions.get(v) == Some(e));
            before != (self.known.len(), self.expressions.len())
        }
        fn close(&mut self) {
            loop {
                let mut inferred = Vec::new();
                for (&out, e) in &self.expressions {
                    let truth = self.known.get(&out).copied();
                    match *e {
                        Expression::Copy(a) | Expression::Not(a) => {
                            let invert = matches!(e, Expression::Not(_));
                            if let Some(t) = truth {
                                inferred.push((a, t ^ invert));
                            }
                            if let Some(&t) = self.known.get(&a) {
                                inferred.push((out, t ^ invert));
                            }
                        }
                        Expression::And(a, b) | Expression::Or(a, b) => {
                            let conjunction = matches!(e, Expression::And(..));
                            let left = self.known.get(&a).copied();
                            let right = self.known.get(&b).copied();
                            if truth == Some(conjunction) {
                                inferred.extend([(a, conjunction), (b, conjunction)]);
                            } else if truth == Some(!conjunction) {
                                if left == Some(conjunction) {
                                    inferred.push((b, !conjunction));
                                }
                                if right == Some(conjunction) {
                                    inferred.push((a, !conjunction));
                                }
                            }
                            if left == Some(!conjunction) || right == Some(!conjunction) {
                                inferred.push((out, !conjunction));
                            } else if left == Some(conjunction) && right == Some(conjunction) {
                                inferred.push((out, conjunction));
                            }
                        }
                        Expression::Project { .. } => {
                            if let Some(t) = truth {
                                for (&peer, expression) in &self.expressions {
                                    if expression == e {
                                        inferred.push((peer, t));
                                    }
                                }
                            }
                        }
                    }
                }
                let before = self.known.len();
                for (value, truth) in inferred {
                    self.known.entry(value).or_insert(truth);
                }
                if self.known.len() == before {
                    break;
                }
            }
        }
        fn define(&mut self, defs: &[u32], expression: Option<&(u32, Expression)>) {
            for value in defs {
                self.known.remove(value);
            }
            self.expressions
                .retain(|out, e| !defs.contains(out) && !defs.iter().any(|&v| e.reads(v)));
            if let Some((output, e)) = expression {
                // An in-place definition reads the preceding version. It cannot
                // establish an equation about the new input home without a phi.
                if !defs.iter().any(|&v| e.reads(v)) {
                    self.expressions.insert(*output, e.clone());
                }
            }
            self.close();
        }
    }

    #[test]
    fn packed_truth_intersections_match_maps_across_word_boundaries() {
        let mut actual = vec![Known::default(); 4];
        let mut expected = vec![BTreeMap::<u32, bool>::new(); 4];
        let boundaries = [0, 1, 31, 32, 63, 64, 255, 256, 1023, 4095, 8191];
        for &value in &boundaries {
            actual[0].insert(value, false);
            expected[0].insert(value, false);
            assert_eq!(actual[0].get(&value), Some(&false));
            actual[0].insert(value, true);
            expected[0].insert(value, true);
            assert_eq!(actual[0].get(&value), Some(&true));
        }
        let mut random = 19u64;
        for step in 0..4096 {
            let mut next = || {
                random = random
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                (random >> 32) as u32
            };
            let current = (next() % 4) as usize;
            let peer = (next() % 4) as usize;
            let operation = next() % 4;
            let value = if step % 2 == 0 {
                boundaries[(next() as usize) % boundaries.len()]
            } else {
                next() % 512
            };
            let truth = next() & 1 != 0;
            match operation {
                0 => {
                    actual[current].insert(value, truth);
                    expected[current].insert(value, truth);
                }
                1 => {
                    actual[current].remove(&value);
                    expected[current].remove(&value);
                }
                2 => {
                    let incoming = actual[peer].clone();
                    let reference = expected[peer].clone();
                    let before = expected[current].len();
                    expected[current].retain(|key, value| reference.get(key) == Some(value));
                    assert_eq!(
                        actual[current].intersect(&incoming),
                        before != expected[current].len()
                    );
                }
                _ => {
                    actual[current] = actual[peer].clone();
                    expected[current] = expected[peer].clone();
                }
            }
            for (actual, expected) in actual.iter().zip(&expected) {
                assert_eq!(actual.to_map(), *expected, "step={step}");
                assert_eq!(actual.len(), expected.len());
                assert_eq!(actual.get(&value), expected.get(&value));
            }
        }
    }

    #[test]
    fn shared_guard_states_match_the_original_analysis_through_updates_and_joins() {
        for seed in 0..64u64 {
            let mut random = seed + 1;
            let mut actual = vec![State::default(); 4];
            let mut expected = vec![ReferenceState::default(); 4];
            for step in 0..1000 {
                let mut next = || {
                    random = random
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    (random >> 32) as u32
                };
                let current = (next() % 4) as usize;
                let peer = (next() % 4) as usize;
                let operation = next() % 12;
                let output = next() % 12;
                let a = next() % 12;
                let b = next() % 12;
                let truth = next() & 1 != 0;
                match operation {
                    0..=4 => {
                        let expression = match operation {
                            0 => Expression::Copy(a),
                            1 => Expression::Not(a),
                            2 => Expression::And(a, b),
                            3 => Expression::Or(a, b),
                            _ => projection(a, b % 3),
                        };
                        actual[current].define(&[output], Some(&(output, expression.clone())));
                        expected[current].define(&[output], Some(&(output, expression)));
                    }
                    5 => {
                        actual[current].define(&[a, b], None);
                        expected[current].define(&[a, b], None);
                    }
                    6 => {
                        actual[current].assume(a, truth);
                        expected[current].known.insert(a, truth);
                    }
                    7 => {
                        actual[current].close();
                        expected[current].close();
                    }
                    8 => {
                        let left = actual[peer].clone();
                        let right = expected[peer].clone();
                        assert_eq!(
                            actual[current].merge(&left),
                            expected[current].merge(&right)
                        );
                    }
                    9 => {
                        actual[current] = actual[peer].clone();
                        expected[current] = expected[peer].clone();
                    }
                    10 => {
                        actual[current].assume(a, truth);
                        actual[current].close();
                        expected[current].known.insert(a, truth);
                        expected[current].close();
                    }
                    _ => {
                        actual[current].define(&[], None);
                        expected[current].define(&[], None);
                    }
                }
                for (actual, expected) in actual.iter().zip(&expected) {
                    assert_eq!(
                        actual.known.to_map(),
                        expected.known,
                        "seed={seed} step={step}"
                    );
                    assert_eq!(
                        actual.expressions.as_ref(),
                        &expected.expressions,
                        "seed={seed} step={step}"
                    );
                    assert_eq!(actual.is_empty(), expected.is_empty());
                    assert_eq!(actual.truth(a), expected.known.get(&a).copied());
                }
            }
        }
    }

    #[test]
    fn unchanged_edges_share_closed_facts_without_mutating_predecessors() {
        let mut parent = State::default();
        parent.define(&[2], Some(&(2, Expression::Copy(1))));
        parent.assume(1, true);
        parent.close();
        let mut child = parent.clone();
        child.define(&[], None);
        child.define(&[999], None);
        assert!(Arc::ptr_eq(&parent.known.words, &child.known.words));
        assert!(Arc::ptr_eq(&parent.expressions, &child.expressions));
        assert!(!child.merge(&parent));
        child.assume(1, false);
        assert_eq!(parent.truth(1), Some(true));
        assert_eq!(child.truth(1), Some(false));
        assert!(!Arc::ptr_eq(&parent.known.words, &child.known.words));
        assert!(Arc::ptr_eq(&parent.expressions, &child.expressions));
    }

    fn projection(source: u32, ordinal: u32) -> Expression {
        Expression::Project {
            source,
            path: vec![(0, ordinal)],
        }
    }

    #[test]
    fn expiration_keeps_pending_inferences_and_new_relations_require_closure() {
        let mut state = State::default();
        state.define(&[2], Some(&(2, Expression::Copy(1))));
        state.assume(2, true);
        // Expiring an unrelated home must not discard the pending assumption.
        state.define(&[9], None);
        assert_eq!(state.truth(1), Some(true));
        state.define(&[1], None);
        assert_eq!(state.truth(1), None);
        assert_eq!(state.truth(2), Some(true));
        state.define(&[3], Some(&(3, Expression::Not(2))));
        assert_eq!(state.truth(3), Some(false));
    }

    #[test]
    fn filtering_known_candidates_preserves_first_inference_on_conflicting_paths() {
        let mut state = State::default();
        for output in [2, 3, 4] {
            state.define(&[output], Some(&(output, projection(1, 0))));
        }
        state.assume(2, true);
        state.assume(3, false);
        state.close();
        assert_eq!(state.truth(2), Some(true));
        assert_eq!(state.truth(3), Some(false));
        assert_eq!(state.truth(4), Some(true));
    }

    #[test]
    fn unrelated_assumptions_keep_closure_and_later_relations_use_their_current_truth() {
        let mut state = State::default();
        state.define(&[2], Some(&(2, Expression::Copy(1))));
        state.assume(9, true);
        assert!(state.closed);
        state.assume(2, true);
        assert!(!state.closed);
        state.assume(9, false);
        assert!(!state.closed);
        state.close();
        assert_eq!(state.truth(1), Some(true));
        state.define(&[3], Some(&(3, Expression::Copy(9))));
        assert_eq!(state.truth(3), Some(false));
    }

    #[test]
    fn shared_relation_index_expires_without_resurrecting_old_projection_peers() {
        let mut parent = State::default();
        for output in [2, 3] {
            parent.define(&[output], Some(&(output, projection(1, 0))));
        }
        parent.assume(2, true);
        parent.close();
        let mut child = parent.clone();
        assert!(Arc::ptr_eq(&parent.index, &child.index));
        child.define(&[1, 2, 3], None);
        assert!(!Arc::ptr_eq(&parent.index, &child.index));
        child.assume(2, true);
        child.close();
        assert_eq!(child.truth(3), None);
        child.define(&[4], Some(&(4, Expression::Copy(2))));
        assert_eq!(child.truth(4), Some(true));
        assert_eq!(parent.truth(3), Some(true));
        assert_eq!(parent.truth(4), None);
    }

    #[test]
    fn joined_relation_indexes_follow_exact_and_partial_intersections() {
        for partial in [false, true] {
            let mut left = State::default();
            for output in [2, 3] {
                left.define(&[output], Some(&(output, projection(1, 0))));
            }
            left.define(&[4], Some(&(4, Expression::Copy(5))));
            left.assume(2, true);
            left.close();
            let mut right = left.clone();
            if partial {
                right.define(&[3, 4], None);
                right.define(&[6], Some(&(6, Expression::Copy(7))));
            } else {
                right.define(&[1], None);
            }
            assert!(left.merge(&right));
            left.define(&[3], None);
            left.close();
            assert_eq!(left.truth(3), None, "partial={partial}");
            left.define(&[1, 2], None);
            left.assume(2, true);
            left.close();
            assert_eq!(left.truth(3), None, "partial={partial}");
        }
    }

    #[test]
    fn initializing_a_relation_index_does_not_change_logical_state_equality() {
        let mut state = State::default();
        state.define(&[2], Some(&(2, Expression::Copy(1))));
        let cold = State {
            known: state.known.clone(),
            expressions: state.expressions.clone(),
            closed: state.closed,
            index: Arc::default(),
        };
        assert!(state.index.get().is_some());
        assert!(cold.index.get().is_none());
        assert_eq!(state, cold);
    }

    #[test]
    fn expiration_matches_full_reclosure_for_every_small_truth_and_definition_subset() {
        for network in 0..12u32 {
            let expressions = Arc::new(
                (0..6u32)
                    .map(|output| {
                        let a = (output * 3 + network + 1) % 6;
                        let b = (output + network + 2) % 6;
                        let expression = match (output + network) % 5 {
                            0 => Expression::Copy(a),
                            1 => Expression::Not(a),
                            2 => Expression::And(a, b),
                            3 => Expression::Or(a, b),
                            _ => projection(a, network % 2),
                        };
                        (output, expression)
                    })
                    .collect::<BTreeMap<_, _>>(),
            );
            for pattern in 0..729usize {
                let mut state = State {
                    expressions: expressions.clone(),
                    ..State::default()
                };
                let mut reference = ReferenceState {
                    known: BTreeMap::new(),
                    expressions: expressions.as_ref().clone(),
                };
                let mut digits = pattern;
                for value in 0..6 {
                    if digits % 3 != 0 {
                        let truth = digits % 3 == 2;
                        state.assume(value, truth);
                        reference.known.insert(value, truth);
                    }
                    digits /= 3;
                }
                state.close();
                reference.close();
                assert_eq!(state.known.to_map(), reference.known);
                for value in 0..8 {
                    for truth in [false, true] {
                        let mut actual = state.clone();
                        let mut expected = reference.clone();
                        actual.assume(value, truth);
                        expected.known.insert(value, truth);
                        actual.close();
                        expected.close();
                        assert_eq!(
                            actual.known.to_map(),
                            expected.known,
                            "network={network} pattern={pattern} assumption=({value},{truth})"
                        );
                    }
                }
                for mask in 0..64 {
                    let defs = (0..6)
                        .filter(|value| mask & (1 << value) != 0)
                        .collect::<Vec<_>>();
                    let mut actual = state.clone();
                    let mut expected = reference.clone();
                    actual.define(&defs, None);
                    expected.define(&defs, None);
                    assert_eq!(
                        actual.known.to_map(),
                        expected.known,
                        "network={network} pattern={pattern} defs={defs:?}"
                    );
                    assert_eq!(actual.expressions.as_ref(), &expected.expressions);
                }
            }
        }
    }
    #[test]
    fn conjunction_truth_reaches_repeated_current_tuple_projections() {
        let mut s = State::default();
        s.define(&[2], Some(&(2, projection(1, 1))));
        s.define(&[4], Some(&(4, Expression::And(2, 3))));
        s.assume(4, true);
        s.close();
        assert_eq!(s.known.get(&2), Some(&true));
        assert_eq!(s.known.get(&3), Some(&true));
        s.define(&[5], Some(&(5, projection(1, 1))));
        assert_eq!(s.known.get(&5), Some(&true));
        s.define(&[6], Some(&(6, projection(1, 0))));
        assert!(!s.known.contains_key(&6));
    }
    #[test]
    fn redefining_a_tuple_keeps_old_boolean_truth_but_expires_its_relation() {
        let mut s = State::default();
        s.define(&[2], Some(&(2, projection(1, 1))));
        s.assume(2, true);
        s.close();
        s.define(&[1], None);
        s.define(&[3], Some(&(3, projection(1, 1))));
        assert_eq!(s.known.get(&2), Some(&true));
        assert!(!s.known.contains_key(&3));
    }
    #[test]
    fn redefined_copy_operands_and_outputs_do_not_share_old_versions() {
        let mut s = State::default();
        s.define(&[2], Some(&(2, Expression::Copy(1))));
        s.define(&[1], None);
        s.assume(2, true);
        s.close();
        assert!(!s.known.contains_key(&1));
        s.define(&[2], None);
        assert!(!s.known.contains_key(&2));
        s.define(&[1], Some(&(1, Expression::Not(1))));
        s.assume(1, true);
        s.close();
        assert!(!s.expressions.contains_key(&1));
    }
    #[test]
    fn false_and_true_or_preserve_both_operand_possibilities() {
        for (e, t) in [(Expression::And(1, 2), false), (Expression::Or(1, 2), true)] {
            let mut s = State::default();
            s.define(&[3], Some(&(3, e)));
            s.assume(3, t);
            s.close();
            assert!(!s.known.contains_key(&1));
            assert!(!s.known.contains_key(&2));
            s.assume(1, !t);
            s.close();
            assert_eq!(s.known.get(&2), Some(&t));
        }
    }
    #[test]
    fn false_or_true_and_and_negated_copies_follow_boolean_semantics() {
        for (e, t) in [(Expression::And(1, 2), true), (Expression::Or(1, 2), false)] {
            let mut s = State::default();
            s.define(&[3], Some(&(3, e)));
            s.define(&[4], Some(&(4, Expression::Not(3))));
            s.define(&[5], Some(&(5, Expression::Copy(4))));
            s.assume(5, !t);
            s.close();
            assert_eq!(s.known.get(&1), Some(&t));
            assert_eq!(s.known.get(&2), Some(&t));
        }
    }
    #[test]
    fn joins_keep_only_relations_true_on_every_incoming_path() {
        let mut left = State::default();
        left.define(&[2], Some(&(2, projection(1, 1))));
        left.assume(2, true);
        let mut right = left.clone();
        right.define(&[1], None);
        assert!(left.merge(&right));
        left.define(&[3], Some(&(3, projection(1, 1))));
        assert!(!left.known.contains_key(&3));
        right.assume(2, false);
        assert!(left.merge(&right));
        assert!(!left.known.contains_key(&2));
    }

    #[test]
    fn sibling_paths_close_with_their_own_truths_and_rejoin_on_common_facts() {
        let mut entry = State::default();
        entry.assume(4, true);
        entry.define(&[2], Some(&(2, projection(1, 1))));
        entry.define(&[3], Some(&(3, Expression::Not(2))));

        let mut left = entry.clone();
        let mut right = entry.clone();
        left.assume(3, true);
        left.close();
        right.assume(3, false);
        right.close();
        assert_eq!(left.known.get(&2), Some(&false));
        assert_eq!(right.known.get(&2), Some(&true));
        assert_eq!(entry.known.get(&2), None);
        assert_eq!(entry.known.get(&3), None);

        assert!(!left.merge(&left.clone()));
        assert!(left.merge(&right));
        left.define(&[], None);
        assert_eq!(left.known.get(&2), None);
        assert_eq!(left.known.get(&3), None);
        assert_eq!(left.known.get(&4), Some(&true));

        left.define(&[1], None);
        left.define(&[5], Some(&(5, projection(1, 1))));
        left.assume(3, true);
        left.close();
        assert_eq!(left.known.get(&2), Some(&false));
        assert_eq!(left.known.get(&5), None);
        assert_eq!(right.known.get(&2), Some(&true));
    }
}
