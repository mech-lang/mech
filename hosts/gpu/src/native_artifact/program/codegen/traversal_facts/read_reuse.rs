//! Reuse an already computed scalar, never a pointer or read permission.
//! Availability is local to one traversal and intersected at every join.
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(in crate::native_artifact::program::codegen) struct Snapshot {
    pub site: usize,
    pub word: usize,
}

impl State {
    pub(super) fn snapshot(&mut self, site: usize, word: usize) {
        let value = self.read(word);
        if let (Some(values), Some(value)) = (&mut self.snapshots, value) {
            values.insert(Snapshot { site, word }, value);
        }
    }

    pub(super) fn snapshot_inputs(&mut self, site: usize, action: &Scalar) {
        if self.snapshots.is_none() {
            return;
        }
        match action {
            Scalar::Copy { source, .. } => self.snapshot(site, source.get() as usize),
            Scalar::Primitive { operands, .. } => {
                for operand in operands {
                    self.snapshot(site, operand.get() as usize);
                }
            }
            Scalar::None => {}
        }
    }

    fn selected(&self, terms: &Terms, mut value: Id) -> Option<bool> {
        let mut invert = false;
        let mut seen = BTreeSet::new();
        loop {
            if !seen.insert(value) {
                return None;
            }
            if let Some(value @ (0 | 1)) = self.constant(terms, value) {
                return Some((value != 0) != invert);
            }
            match terms.terms[value] {
                Expr::Not(inner) => {
                    invert = !invert;
                    value = inner;
                }
                Expr::Compare(relation, a, b) => {
                    return self.truth(terms, relation, a, b).map(|v| v != invert);
                }
                _ => return None,
            }
        }
    }

    fn is_read(&self, terms: &Terms, value: Id, array: usize, index: Id) -> bool {
        use write_changes::Value;
        let mut needed = BTreeSet::new();
        let mut pending = vec![value];
        while let Some(value) = pending.pop() {
            if !needed.insert(value) {
                continue;
            }
            match terms.changes.value(value) {
                Some(Value::Scalar(op, args)) if unsigned_cast(*op) => {
                    pending.extend(args);
                }
                Some(Value::Select { condition, yes, no }) => {
                    match self.selected(terms, *condition) {
                        Some(true) => pending.push(*yes),
                        Some(false) => pending.push(*no),
                        None => pending.extend([*yes, *no]),
                    }
                }
                _ => {}
            }
        }
        let mut equal = BTreeMap::new();
        for value in needed {
            let get = |id| equal.get(&id).copied().unwrap_or(false);
            let current = match terms.changes.value(value) {
                Some(Value::Read {
                    array: a,
                    index: i,
                    version,
                }) => {
                    *a == array
                        && *version == self.content_version
                        && write_changes::same_index(terms, *i, index)
                }
                // Narrowing has already succeeded at its original lexical site.
                // These casts preserve the unsigned word on that successful path.
                Some(Value::Scalar(op, args)) if unsigned_cast(*op) && args.len() == 1 => {
                    get(args[0])
                }
                Some(Value::Select { condition, yes, no }) => {
                    match self.selected(terms, *condition) {
                        Some(true) => get(*yes),
                        Some(false) => get(*no),
                        None => get(*yes) && get(*no),
                    }
                }
                _ => false,
            };
            equal.insert(value, current);
        }
        equal.get(&value).copied().unwrap_or(false)
    }
}

fn unsigned_cast(op: Op) -> bool {
    use Op::*;
    matches!(
        op,
        U8ToU16
            | U8ToU32
            | U8ToU64
            | U16ToU8
            | U16ToU32
            | U16ToU64
            | U32ToU8
            | U32ToU16
            | U32ToU64
            | U64ToU8
            | U64ToU16
            | U64ToU32
    )
}

fn input_words(action: &Scalar) -> Vec<usize> {
    match action {
        Scalar::Copy { source, .. } => vec![source.get() as usize],
        Scalar::Primitive { operands, .. } => operands.iter().map(|v| v.get() as usize).collect(),
        Scalar::None => Vec::new(),
    }
}

fn read_related(terms: &Terms, root: Id) -> bool {
    let mut pending = vec![root];
    let mut seen = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if !seen.insert(id) {
            continue;
        }
        match terms.changes.value(id) {
            Some(write_changes::Value::Read { .. }) => return true,
            Some(write_changes::Value::Scalar(op, args)) if unsigned_cast(*op) => {
                pending.extend(args)
            }
            Some(write_changes::Value::Select { yes, no, .. }) => pending.extend([*yes, *no]),
            _ => {}
        }
    }
    false
}

// These are two already evaluated scalar values. Compare their recorded read
// epochs with each other, not with current memory: an old scalar stays valid
// after a write, while a new read receives a different epoch. All intervening
// source bounds/arithmetic/assertion checks still execute at their old sites.
fn equivalent(state: &State, terms: &Terms, a: Id, b: Id) -> bool {
    use write_changes::Value;
    let key = |a, b| if a >= b { (a, b) } else { (b, a) };
    let mut needed = BTreeSet::new();
    let mut pending = vec![key(a, b)];
    let children = |a, b| -> Vec<(Id, Id)> {
        if a == b {
            return Vec::new();
        }
        if let Some(Value::Scalar(op, args)) = terms.changes.value(a) {
            if unsigned_cast(*op) && args.len() == 1 {
                return vec![key(args[0], b)];
            }
        }
        if let Some(Value::Scalar(op, args)) = terms.changes.value(b) {
            if unsigned_cast(*op) && args.len() == 1 {
                return vec![key(a, args[0])];
            }
        }
        let left = terms.changes.value(a);
        let right = terms.changes.value(b);
        if let Some(Value::Select { condition, yes, no }) = left {
            if let Some(value) = state.selected(terms, *condition) {
                return vec![key(if value { *yes } else { *no }, b)];
            }
        }
        if let Some(Value::Select { condition, yes, no }) = right {
            if let Some(value) = state.selected(terms, *condition) {
                return vec![key(a, if value { *yes } else { *no })];
            }
        }
        match (left, right) {
            (
                Some(Value::Select {
                    condition: x,
                    yes: a,
                    no: b,
                }),
                Some(Value::Select {
                    condition: y,
                    yes: c,
                    no: d,
                }),
            ) if x == y => vec![key(*a, *c), key(*b, *d)],
            _ => Vec::new(),
        }
    };
    while let Some(pair) = pending.pop() {
        if needed.insert(pair) {
            pending.extend(children(pair.0, pair.1));
        }
    }
    let mut known = BTreeMap::new();
    for (a, b) in needed {
        let dependencies = children(a, b);
        let same = if a == b {
            true
        } else if let (Some(x), Some(y)) = (terms.value(a), terms.value(b)) {
            x == y
        } else if !dependencies.is_empty() {
            dependencies.iter().all(|p| known.get(p) == Some(&true))
        } else {
            matches!((terms.changes.value(a),terms.changes.value(b)),
            (Some(Value::Read{array:x,index:i,version:v}),Some(Value::Read{array:y,index:j,version:w}))
            if x==y && v==w && write_changes::same_index(terms,*i,*j))
        };
        known.insert((a, b), same);
    }
    known.get(&key(a, b)) == Some(&true)
}

pub(super) fn operands(
    states: &[Option<State>],
    terms: &Terms,
    action: &Scalar,
) -> BTreeMap<usize, Snapshot> {
    let mut result = BTreeMap::new();
    if states
        .iter()
        .flatten()
        .all(|state| state.snapshots.is_none())
    {
        return result;
    }
    let mut related = BTreeMap::new();
    for word in input_words(action) {
        let mut common: Option<BTreeSet<Snapshot>> = None;
        for state in states.iter().flatten() {
            let Some(values) = &state.snapshots else {
                return BTreeMap::new();
            };
            let Some(current) = state.read(word) else {
                return BTreeMap::new();
            };
            if !*related
                .entry(current)
                .or_insert_with(|| read_related(terms, current))
            {
                common = Some(BTreeSet::new());
                break;
            }
            let mut compared = BTreeMap::new();
            let set = values
                .iter()
                .filter_map(|(&snapshot, &prior)| {
                    (current != prior
                        && *related
                            .entry(prior)
                            .or_insert_with(|| read_related(terms, prior))
                        && *compared
                            .entry(prior)
                            .or_insert_with(|| equivalent(state, terms, current, prior)))
                    .then_some(snapshot)
                })
                .collect::<BTreeSet<_>>();
            if let Some(common) = &mut common {
                common.retain(|p| set.contains(p));
            } else {
                common = Some(set);
            }
        }
        if let Some(snapshot) = common.and_then(|v| v.into_iter().next()) {
            result.insert(word, snapshot);
        }
    }
    result
}

pub(super) fn find(
    states: &[Option<State>],
    terms: &Terms,
    array: usize,
    index: u32,
) -> Option<Snapshot> {
    let mut common: Option<BTreeSet<Snapshot>> = None;
    for state in states.iter().flatten() {
        let values = state.snapshots.as_ref()?;
        let index = state.read(index as usize)?;
        let mut compared = BTreeMap::new();
        let current = values
            .iter()
            .filter_map(|(&snapshot, &value)| {
                (*compared
                    .entry(value)
                    .or_insert_with(|| state.is_read(terms, value, array, index)))
                .then_some(snapshot)
            })
            .collect::<BTreeSet<_>>();
        if let Some(common) = &mut common {
            common.retain(|value| current.contains(value));
        } else {
            common = Some(current);
        }
    }
    common?.into_iter().next()
}

impl Proof {
    pub(super) fn extend_reuses(&mut self, other: &mut Self) {
        self.ambiguous_reuses.append(&mut other.ambiguous_reuses);
        for (site, value) in std::mem::take(&mut other.reused_reads) {
            if self
                .reused_reads
                .get(&site)
                .is_some_and(|old| *old != value)
            {
                self.ambiguous_reuses.insert(site);
            } else {
                self.reused_reads.insert(site, value);
            }
        }
        self.reused_reads
            .retain(|site, _| !self.ambiguous_reuses.contains(site));
        self.ambiguous_operands
            .append(&mut other.ambiguous_operands);
        for (key, value) in std::mem::take(&mut other.reused_operands) {
            if self
                .reused_operands
                .get(&key)
                .is_some_and(|old| *old != value)
            {
                self.ambiguous_operands.insert(key);
            } else {
                self.reused_operands.insert(key, value);
            }
        }
        self.reused_operands
            .retain(|key, _| !self.ambiguous_operands.contains(key));
    }

    pub fn reused_read(&self, site: usize) -> Option<Snapshot> {
        self.reused_reads.get(&site).copied()
    }

    pub fn snapshots_at(&self, site: usize) -> BTreeSet<Snapshot> {
        self.reused_reads
            .values()
            .chain(self.reused_operands.values())
            .filter(|s| s.site == site)
            .copied()
            .collect()
    }

    pub fn reused_operand(&self, site: usize, word: usize) -> Option<Snapshot> {
        self.reused_operands.get(&(site, word)).copied()
    }
}

#[cfg(test)]
#[path = "read_reuse_tests.rs"]
mod tests;
