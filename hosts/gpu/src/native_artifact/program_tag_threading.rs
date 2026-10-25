//! Bounded product CFG for a closed enum-state Tail. Only the actual current
//! tag survives a latch. Integer ranges, predicates about mutable words, and
//! assumptions about the initializer do not cross an iteration boundary.
use super::*;
use mech_engine::native_artifact::NativeScalarOp as Op;

const MAX_SITES: usize = 256;
const MAX_WORDS: usize = 128;
const MAX_VARIANTS: usize = 8;
const MAX_NODES: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Fact {
    Unknown,
    Constant(u64),
    Other,
}
#[derive(Clone, Copy)]
enum Input {
    Word(usize),
    Constant(u64),
    Unknown,
}
#[derive(Clone)]
enum Expr {
    Copy(Input),
    Equal(Input, u32),
    Not(Input),
}
impl Expr {
    fn input(&self) -> Input {
        match *self {
            Self::Copy(i) | Self::Equal(i, _) | Self::Not(i) => i,
        }
    }
}
#[derive(Clone, Default)]
struct Transfer {
    writes: Vec<(usize, Expr)>,
}
impl Transfer {
    fn apply(&self, state: &mut [Fact], positions: &BTreeMap<usize, usize>, variants: &[u32]) {
        // Aggregate copies are simultaneous: all reads use the old state.
        let values = self
            .writes
            .iter()
            .filter_map(|(slot, expr)| {
                let position = positions.get(slot).copied()?;
                let input = match expr.input() {
                    Input::Constant(word) => Fact::Constant(word),
                    Input::Word(slot) => positions.get(&slot).map_or(Fact::Unknown, |&i| state[i]),
                    Input::Unknown => Fact::Unknown,
                };
                let value = match expr {
                    Expr::Copy(_) => input,
                    Expr::Equal(_, ordinal) => match input {
                        Fact::Constant(word) => {
                            Fact::Constant(u64::from(word == u64::from(*ordinal)))
                        }
                        Fact::Other if variants.contains(ordinal) => Fact::Constant(0),
                        _ => Fact::Unknown,
                    },
                    // The machine primitive is XOR 1, not a general zero test.
                    Expr::Not(_) => match input {
                        Fact::Constant(word @ (0 | 1)) => Fact::Constant(word ^ 1),
                        _ => Fact::Unknown,
                    },
                };
                Some((position, value))
            })
            .collect::<Vec<_>>();
        for (position, value) in values {
            state[position] = value;
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) enum Target {
    Node(usize),
    Exit,
}
#[derive(Clone, Debug)]
pub(in crate::native_artifact) enum Edge {
    Jump(Target),
    Branch {
        yes: Target,
        no: Target,
    },
    Dispatch {
        cases: Vec<(u32, Target)>,
        other: Target,
    },
    Reject,
}
pub(in crate::native_artifact) struct Node {
    pub site: usize,
    pub edge: Edge,
    /// A header may rematerialize the exact value proved at its incoming
    /// edge. This makes the machine SSA fact explicit without changing it.
    pub header_tag: Option<u32>,
    pub folded_branch: bool,
    pub known_latch: bool,
}
pub(in crate::native_artifact) struct Graph {
    pub carrier_word: usize,
    pub variants: Vec<u32>,
    pub nodes: Vec<Node>,
}

fn aggregate_transfer(layout: &NativeScalarAggregates, action: &Aggregate) -> Option<Transfer> {
    let input = |value, index| field(layout, value, index).map(Input::Word);
    let (output, values) = match action {
        Aggregate::Pack { output, words } => (
            *output,
            words
                .iter()
                .map(|word| {
                    Some(match word {
                        Word::Scalar(value) => Input::Word(value.get() as usize),
                        Word::Field(value, index) => input(*value, *index)?,
                        Word::Constant(word) => Input::Constant(*word),
                    })
                })
                .collect::<Option<Vec<_>>>()?,
        ),
        Aggregate::Copy { source, output } => (
            *output,
            (0..layout.slot(*output)?.1)
                .map(|i| input(*source, i))
                .collect::<Option<Vec<_>>>()?,
        ),
        Aggregate::Project {
            source,
            offset,
            output,
            ..
        } => (
            *output,
            (0..layout.slot(*output)?.1)
                .map(|i| input(*source, offset.checked_add(i)?))
                .collect::<Option<Vec<_>>>()?,
        ),
        Aggregate::Observe {
            source,
            offset,
            ordinal,
            output,
            ..
        } => {
            let value = input(*source, *offset)?;
            let expr = ordinal.map_or(Expr::Copy(value), |ordinal| Expr::Equal(value, ordinal));
            return Some(Transfer {
                writes: vec![(output.get() as usize, expr)],
            });
        }
    };
    let (base, count) = layout.slot(output)?;
    if values.len() != count {
        return None;
    }
    Some(Transfer {
        writes: values
            .into_iter()
            .enumerate()
            .map(|(i, value)| (base + i, Expr::Copy(value)))
            .collect(),
    })
}

/// Independent whitelist: private packed callbacks mutate their Frame, never
/// the source SSA word array. An unknown host action declines this proof.
fn transfer(
    code: CodePlan<'_>,
    closed: &Layout,
    id: usize,
    owner: &NativeMapOwnerWords,
    aggregates: &NativeScalarAggregates,
    site: usize,
) -> Option<Transfer> {
    let owner_index = closed.regions[id].map?;
    if let Some((layout, action)) = aggregate_at(owner, aggregates, owner_index, site) {
        return aggregate_transfer(layout, action);
    }
    if let Some(span) = owner.span_site(site).map(|i| &owner.spans()[i]) {
        return (span.region() == owner_index).then(Transfer::default);
    }
    if let Action::LibraryCall {
        template, output, ..
    } = code.blocks()[site].action()
    {
        return (owner.call_region(site) == Some(owner_index)
            && matches!(
                template.primitive(),
                P::MapContainsKey | P::MapKeyAccess | P::MapInsert
            ))
        .then(|| Transfer {
            writes: vec![(output.get() as usize, Expr::Copy(Input::Unknown))],
        });
    }
    scalar_transfer(code, site)
}
fn scalar_transfer(code: CodePlan<'_>, site: usize) -> Option<Transfer> {
    Some(match code.blocks()[site].action() {
        Action::Scalar(Scalar::None) => Transfer::default(),
        Action::Scalar(Scalar::Copy { source, output }) if code.kind(*output).is_some() => {
            Transfer {
                writes: vec![(
                    output.get() as usize,
                    Expr::Copy(Input::Word(source.get() as usize)),
                )],
            }
        }
        Action::Scalar(Scalar::Primitive {
            op,
            operands,
            output,
            ..
        }) => {
            let expr = if *op == Op::BoolNot && operands.len() == 1 {
                Expr::Not(Input::Word(operands[0].get() as usize))
            } else {
                Expr::Copy(Input::Unknown)
            };
            Transfer {
                writes: vec![(output.get() as usize, expr)],
            }
        }
        _ => return None,
    })
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    site: usize,
    state: Vec<Fact>,
}
fn literal_equal(a: Fact, b: Fact, variants: &[u32]) -> Option<bool> {
    match (a, b) {
        (Fact::Constant(a), Fact::Constant(b)) => Some(a == b),
        (Fact::Other, Fact::Constant(tag)) | (Fact::Constant(tag), Fact::Other)
            if variants.iter().any(|&v| u64::from(v) == tag) =>
        {
            Some(false)
        }
        _ => None,
    }
}
fn next_header(initial: &[Fact], carrier: usize, tag: Fact, variants: &[u32]) -> Vec<Fact> {
    let tag = match tag {
        Fact::Constant(tag) if variants.iter().any(|&v| u64::from(v) == tag) => Fact::Constant(tag),
        Fact::Constant(_) | Fact::Other => Fact::Other,
        Fact::Unknown => Fact::Unknown,
    };
    let mut next = initial.to_vec();
    next[carrier] = tag;
    next
}
struct Builder {
    keys: Vec<Key>,
    interned: BTreeMap<Key, usize>,
    sites: BTreeSet<usize>,
    exit: usize,
}
impl Builder {
    fn intern(&mut self, site: usize, state: Vec<Fact>) -> Option<Target> {
        if site == self.exit {
            return Some(Target::Exit);
        }
        if !self.sites.contains(&site) {
            return None;
        }
        let key = Key { site, state };
        if let Some(&id) = self.interned.get(&key) {
            return Some(Target::Node(id));
        }
        if self.keys.len() == MAX_NODES {
            return None;
        }
        let id = self.keys.len();
        self.keys.push(key.clone());
        self.interned.insert(key, id);
        Some(Target::Node(id))
    }
}

pub(super) fn derive(
    code: CodePlan<'_>,
    closed: &Layout,
    id: usize,
    owner: &NativeMapOwnerWords,
    aggregates: &NativeScalarAggregates,
) -> Option<Graph> {
    let region = &closed.regions[id];
    region.packed.as_ref()?;
    let carrier_word = field(owner.aggregates(), region.tail.carrier(), 0)?;
    let transfers = closed
        .sites
        .iter()
        .enumerate()
        .filter(|(_, here)| **here == Some(id))
        .map(|(site, _)| Some((site, transfer(code, closed, id, owner, aggregates, site)?)))
        .collect::<Option<BTreeMap<_, _>>>()?;
    derive_from(code, closed, id, carrier_word, transfers)
}

/// The same bounded product graph over a proved callback-free scalar Tail.
/// Borrowed actions can change their declared output, never a carrier home.
/// Unknown host actions decline the graph rather than inventing a write set.
pub(in crate::native_artifact) fn derive_scalar(
    code: CodePlan<'_>,
    closed: &Layout,
    id: usize,
    aggregates: &NativeScalarAggregates,
    borrowed: &super::super::super::program_borrowed_layout::Layout,
) -> Option<Graph> {
    let region = &closed.regions[id];
    if region.map.is_some() {
        return None;
    }
    let carrier_word = field(aggregates, region.tail.carrier(), 0)?;
    let transfers = closed
        .sites
        .iter()
        .enumerate()
        .filter(|(_, here)| **here == Some(id))
        .map(|(site, _)| {
            let transfer = if let Some(action) = aggregates.action(site) {
                aggregate_transfer(aggregates, action)?
            } else if let Some((borrow, action)) = borrowed.action(site) {
                if Some(borrow) != region.borrowed {
                    return None;
                }
                Transfer {
                    writes: vec![(action.output().get() as usize, Expr::Copy(Input::Unknown))],
                }
            } else {
                scalar_transfer(code, site)?
            };
            Some((site, transfer))
        })
        .collect::<Option<BTreeMap<_, _>>>()?;
    derive_from(code, closed, id, carrier_word, transfers)
}

fn derive_from(
    code: CodePlan<'_>,
    closed: &Layout,
    id: usize,
    carrier_word: usize,
    transfers: BTreeMap<usize, Transfer>,
) -> Option<Graph> {
    let region = &closed.regions[id];
    let header = region.tail.header() as usize;
    let latch = region.tail.latch() as usize;
    let sites = transfers.keys().copied().collect::<BTreeSet<_>>();
    if sites.len() > MAX_SITES {
        return None;
    }
    if !matches!(code.blocks()[header].action(), Action::Scalar(Scalar::None)) {
        return None;
    }
    let Term::TailHeader { body, .. } = code.blocks()[header].terminator() else {
        return None;
    };
    if !matches!(code.blocks()[latch].terminator(), Term::Scalar(Branch::Jump(next)) if next as usize == header)
    {
        return None;
    }
    if !transfers.get(&header)?.writes.is_empty() {
        return None;
    }
    // Keep exactly the connected tag/copy/predicate words. Arithmetic is
    // opaque and introduces no dependency on indices, widths or map counts.
    let mut tracked = BTreeSet::from([carrier_word]);
    loop {
        let before = tracked.len();
        for transfer in transfers.values() {
            for &(output, ref expr) in &transfer.writes {
                let Input::Word(input) = expr.input() else {
                    continue;
                };
                if tracked.contains(&output) || tracked.contains(&input) {
                    tracked.insert(output);
                    tracked.insert(input);
                }
            }
        }
        for &site in &sites {
            if let Term::Scalar(Branch::LiteralBranch { left, right, .. }) =
                code.blocks()[site].terminator()
            {
                let (left, right) = (left.get() as usize, right.get() as usize);
                if tracked.contains(&left) || tracked.contains(&right) {
                    tracked.insert(left);
                    tracked.insert(right);
                }
            }
        }
        if tracked.len() > MAX_WORDS {
            return None;
        }
        if tracked.len() == before {
            break;
        }
    }
    let positions = tracked
        .iter()
        .enumerate()
        .map(|(i, &slot)| (slot, i))
        .collect::<BTreeMap<_, _>>();
    let carrier = positions[&carrier_word];
    let mut variants = BTreeSet::new();
    for transfer in transfers.values() {
        for (_, expr) in &transfer.writes {
            if let Expr::Equal(Input::Word(input), ordinal) = expr {
                if tracked.contains(input) {
                    variants.insert(*ordinal);
                }
            }
        }
    }
    if variants.is_empty() || variants.len() > MAX_VARIANTS {
        return None;
    }
    let variants = variants.into_iter().collect::<Vec<_>>();
    let mut initial = vec![Fact::Unknown; tracked.len()];
    for index in 0..code.constant_count() {
        let (value, raw) = code.constant(index).ok()?;
        if let (Some(&position), Some(raw)) = (positions.get(&(value.get() as usize)), raw) {
            initial[position] = Fact::Constant(raw);
        }
    }
    // Never infer the first carrier from the initializer's source spelling.
    initial[carrier] = Fact::Unknown;
    let mut builder = Builder {
        keys: Vec::new(),
        interned: BTreeMap::new(),
        sites,
        exit: region.tail.exit() as usize,
    };
    builder.intern(header, initial.clone())?;
    let mut nodes = Vec::new();
    while nodes.len() < builder.keys.len() {
        let Key { site, mut state } = builder.keys[nodes.len()].clone();
        let header_tag = if site == header {
            match state[carrier] {
                Fact::Constant(tag) => Some(u32::try_from(tag).ok()?),
                _ => None,
            }
        } else {
            None
        };
        transfers[&site].apply(&mut state, &positions, &variants);
        let read = |slot: usize| {
            positions
                .get(&slot)
                .map_or(Fact::Unknown, |&index| state[index])
        };
        let mut folded_branch = false;
        let mut known_latch = false;
        let edge = if site == header && state[carrier] == Fact::Unknown {
            let mut cases = Vec::new();
            for &ordinal in &variants {
                let mut partition = initial.clone();
                partition[carrier] = Fact::Constant(u64::from(ordinal));
                cases.push((ordinal, builder.intern(body as usize, partition)?));
            }
            let mut partition = initial.clone();
            partition[carrier] = Fact::Other;
            Edge::Dispatch {
                cases,
                other: builder.intern(body as usize, partition)?,
            }
        } else if site == latch {
            // This reset occurs after the latch's simultaneous source copy.
            // No predicate, old index, range or arithmetic fact survives.
            let next = next_header(&initial, carrier, state[carrier], &variants);
            known_latch = next[carrier] != Fact::Unknown;
            Edge::Jump(builder.intern(header, next)?)
        } else {
            match code.blocks()[site].terminator() {
                Term::TailHeader { body, .. } | Term::Scalar(Branch::Jump(body)) => {
                    Edge::Jump(builder.intern(body as usize, state.clone())?)
                }
                Term::Scalar(Branch::Branch { condition, yes, no }) => {
                    match read(condition.get() as usize) {
                        Fact::Constant(word) => {
                            folded_branch = true;
                            Edge::Jump(builder.intern(
                                (if word != 0 { yes } else { no }) as usize,
                                state.clone(),
                            )?)
                        }
                        _ => Edge::Branch {
                            yes: builder.intern(yes as usize, state.clone())?,
                            no: builder.intern(no as usize, state.clone())?,
                        },
                    }
                }
                Term::Scalar(Branch::LiteralBranch {
                    left,
                    right,
                    yes,
                    no,
                }) => {
                    let equal = if code.kind(left)? == NativeScalarKind::F64 {
                        None
                    } else {
                        literal_equal(
                            read(left.get() as usize),
                            read(right.get() as usize),
                            &variants,
                        )
                    };
                    if let Some(equal) = equal {
                        folded_branch = true;
                        Edge::Jump(
                            builder
                                .intern((if equal { yes } else { no }) as usize, state.clone())?,
                        )
                    } else {
                        Edge::Branch {
                            yes: builder.intern(yes as usize, state.clone())?,
                            no: builder.intern(no as usize, state.clone())?,
                        }
                    }
                }
                Term::Scalar(Branch::Reject) => Edge::Reject,
                _ => return None,
            }
        };
        nodes.push(Node {
            site,
            edge,
            header_tag,
            folded_branch,
            known_latch,
        });
    }
    if !nodes.iter().any(|node| node.folded_branch) || !nodes.iter().any(|node| node.known_latch) {
        return None;
    }
    Some(Graph {
        carrier_word,
        variants,
        nodes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_carrier_copies_read_old_words_and_observations_keep_evaluated_values() {
        let positions = BTreeMap::from([(10, 0), (11, 1), (12, 2), (13, 3)]);
        let mut state = vec![
            Fact::Constant(0),
            Fact::Constant(1),
            Fact::Unknown,
            Fact::Unknown,
        ];
        Transfer {
            writes: vec![
                (12, Expr::Equal(Input::Word(10), 0)),
                (13, Expr::Copy(Input::Word(10))),
            ],
        }
        .apply(&mut state, &positions, &[0, 1]);
        Transfer {
            writes: vec![
                (10, Expr::Copy(Input::Word(11))),
                (11, Expr::Copy(Input::Word(10))),
            ],
        }
        .apply(&mut state, &positions, &[0, 1]);
        assert_eq!(
            state,
            vec![
                Fact::Constant(1),
                Fact::Constant(0),
                Fact::Constant(1),
                Fact::Constant(0)
            ]
        );
        // A saved predicate and alias refer to the observed value, not the
        // current carrier slot. An opaque redefinition kills only its output.
        Transfer {
            writes: vec![(10, Expr::Copy(Input::Unknown))],
        }
        .apply(&mut state, &positions, &[0, 1]);
        assert_eq!(
            state,
            vec![
                Fact::Unknown,
                Fact::Constant(0),
                Fact::Constant(1),
                Fact::Constant(0)
            ]
        );
    }

    #[test]
    fn invalid_remainder_and_noncanonical_boolean_words_stay_conservative() {
        let positions = BTreeMap::from([(0, 0), (1, 1), (2, 2), (3, 3)]);
        let mut state = vec![Fact::Other, Fact::Unknown, Fact::Unknown, Fact::Unknown];
        Transfer {
            writes: vec![
                (1, Expr::Equal(Input::Word(0), 0)),
                (2, Expr::Equal(Input::Word(0), 7)),
                (3, Expr::Not(Input::Constant(2))),
            ],
        }
        .apply(&mut state, &positions, &[0, 1]);
        assert_eq!(
            state,
            vec![Fact::Other, Fact::Constant(0), Fact::Unknown, Fact::Unknown]
        );
        Transfer {
            writes: vec![(2, Expr::Not(Input::Word(1)))],
        }
        .apply(&mut state, &positions, &[0, 1]);
        assert_eq!(state[2], Fact::Constant(1));
        assert_eq!(literal_equal(Fact::Other, Fact::Other, &[0, 1]), None);
        assert_eq!(
            literal_equal(Fact::Other, Fact::Constant(0), &[0, 1]),
            Some(false)
        );
        assert_eq!(literal_equal(Fact::Constant(7), Fact::Other, &[0, 1]), None);
    }

    #[test]
    fn latch_reset_uses_post_copy_tag_and_discards_every_other_traversal_fact() {
        let initial = vec![Fact::Unknown, Fact::Constant(19), Fact::Unknown];
        let positions = BTreeMap::from([(0, 0), (1, 1), (2, 2)]);
        let mut completed = vec![Fact::Constant(0), Fact::Constant(19), Fact::Constant(1)];
        Transfer {
            writes: vec![(0, Expr::Copy(Input::Word(2)))],
        }
        .apply(&mut completed, &positions, &[0, 1]);
        assert_eq!(
            next_header(&initial, 0, completed[0], &[0, 1]),
            vec![Fact::Constant(1), Fact::Constant(19), Fact::Unknown]
        );
        assert_eq!(next_header(&initial, 0, Fact::Unknown, &[0, 1]), initial);
        assert_eq!(
            next_header(&initial, 0, Fact::Constant(u64::MAX), &[0, 1]),
            vec![Fact::Other, Fact::Constant(19), Fact::Unknown]
        );
    }

    #[test]
    fn product_interning_bounds_adversarial_state_expansion_without_merging_facts() {
        let mut builder = Builder {
            keys: Vec::new(),
            interned: BTreeMap::new(),
            sites: BTreeSet::from([7]),
            exit: 8,
        };
        for word in 0..MAX_NODES {
            assert_eq!(
                builder.intern(7, vec![Fact::Constant(word as u64)]),
                Some(Target::Node(word))
            );
        }
        assert_eq!(
            builder.intern(7, vec![Fact::Constant(0)]),
            Some(Target::Node(0))
        );
        assert_eq!(builder.intern(7, vec![Fact::Unknown]), None);
        assert_eq!(builder.intern(8, vec![Fact::Unknown]), Some(Target::Exit));
        assert_eq!(builder.intern(9, vec![Fact::Unknown]), None);
        assert_eq!(builder.keys.len(), MAX_NODES);
    }
}
