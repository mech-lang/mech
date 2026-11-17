//! Diagnostic scopes for selected BestChoice replays. Compiled only by the
//! private profile harness. Timing and work census use separate binaries.
use super::{Attempt, GrammarExpression, GrammarState, Parser, RuleId};
use crate::document::parser::{ParserCheckpoint, canonical_rule_name, tree_cache::CacheWork};
use crate::document::{ParseStats, TextSize};
use std::{cell::RefCell, collections::BTreeMap, time::Instant};

const TIME: bool = cfg!(best_choice_probe = "time");

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
struct Family {
    owner: Option<RuleId>,
    alternative: Option<RuleId>,
}

impl Family {
    fn new(parser: &Parser<'_>, expression: &GrammarExpression) -> Self {
        Self {
            owner: parser.current_rule(),
            alternative: match expression {
                GrammarExpression::Rule(rule) => Some(*rule),
                _ => None,
            },
        }
    }

    fn labels(self) -> (&'static str, &'static str) {
        (
            self.owner
                .and_then(canonical_rule_name)
                .unwrap_or("anonymous-choice"),
            self.alternative
                .and_then(canonical_rule_name)
                .unwrap_or("non-rule"),
        )
    }
}

#[derive(Default)]
struct Time {
    exclusive: u64,
    outermost: u64,
    inclusive_union: u64,
}

#[derive(Default)]
struct Clock {
    begin: Option<Instant>,
    previous: Option<Instant>,
    active: Vec<Family>,
    times: BTreeMap<Family, Time>,
    outside: u64,
}

impl Clock {
    fn split(&mut self) {
        let Some(previous) = self.previous else {
            return;
        };
        let nanos = previous.elapsed().as_nanos() as u64;
        let Some(last) = self.active.last() else {
            self.outside += nanos;
            return;
        };
        self.times.entry(*last).or_default().exclusive += nanos;
        self.times.entry(self.active[0]).or_default().outermost += nanos;
        // A family can nest in itself through an embedded document. Its union
        // is the elapsed window during which at least one such replay ran.
        for (index, &family) in self.active.iter().enumerate() {
            if !self.active[..index].contains(&family) {
                self.times.entry(family).or_default().inclusive_union += nanos;
            }
        }
    }

    fn change(&mut self, change: impl FnOnce(&mut Vec<Family>)) {
        self.split();
        change(&mut self.active);
        if self.begin.is_some() {
            self.previous = Some(Instant::now());
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct Context {
    cursor_end: TextSize,
    context_end: TextSize,
    consume_frontier: bool,
    context_frontier: bool,
    recovery_allowed: bool,
    delimiter: Option<RuleId>,
}

#[derive(Clone, Copy)]
struct Snapshot {
    checkpoint: ParserCheckpoint,
    context: Context,
    stats: ParseStats,
    cache: CacheWork,
    work: u64,
    fuel: u64,
    nodes: u64,
    tokens: u64,
    diagnostics: u64,
    rewinds: u64,
    rewound_bytes: u64,
    cached_bytes: u64,
    peaks: [usize; 4],
    one_piece_batch: bool,
    pending: bool,
    warm_origin: bool,
}

impl Snapshot {
    fn capture(parser: &Parser<'_>, state: GrammarState, work: u64, input_final: bool) -> Self {
        let p = &parser.state;
        let ctx_end = parser.cursor().context_end();
        Self {
            checkpoint: parser.checkpoint(),
            context: Context {
                cursor_end: parser.cursor().end(),
                context_end: ctx_end,
                consume_frontier: p.cursor_frontier,
                context_frontier: p.context_frontier,
                recovery_allowed: p.allow_consuming_recovery,
                delimiter: state.codeblock_delimiter,
            },
            stats: p.stats,
            cache: p.tree_cache.total_work(),
            work,
            fuel: p.fuel,
            nodes: parser.ids.next_node(),
            tokens: parser.ids.next_token(),
            diagnostics: parser.ids.next_diagnostic(),
            rewinds: p.rewinds,
            rewound_bytes: p.rewound_bytes,
            cached_bytes: p.cached_replay_bytes,
            peaks: [
                p.peak_events,
                p.peak_open_markers,
                p.rules.peak_depth(),
                p.tree_cache.peak_pending,
            ],
            one_piece_batch: input_final
                && p.tree_cache.one_shot
                && parser.source().piece_count() == 1,
            pending: p.resource_finalizing || p.resource_found.is_some() || p.tree_cache.pending(),
            warm_origin: p.grapheme_tail.as_ref().is_some_and(|tail| {
                tail.resumes_at(parser.offset(), (!p.context_frontier).then_some(ctx_end))
            }),
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
struct Effect {
    end: ParserCheckpoint,
    context: Context,
    work: u64,
    charges: u64,
    steps: u64,
    nodes: u64,
    tokens: u64,
    rewinds: u64,
    rewound_bytes: u64,
    cached_bytes: u64,
    cache: [u64; 5],
}

struct Outcome {
    effect: Effect,
    beginning: Snapshot,
    clean: bool,
    balanced: bool,
    peaks_raised: bool,
}

impl Outcome {
    fn new(beginning: Snapshot, end: Snapshot) -> Self {
        let b = beginning;
        Self {
            effect: Effect {
                end: end.checkpoint,
                context: end.context,
                work: end.work - b.work,
                charges: b.fuel - end.fuel,
                steps: end.stats.parser_steps - b.stats.parser_steps,
                nodes: end.nodes - b.nodes,
                tokens: end.tokens - b.tokens,
                rewinds: end.rewinds - b.rewinds,
                rewound_bytes: end.rewound_bytes - b.rewound_bytes,
                cached_bytes: end.cached_bytes - b.cached_bytes,
                cache: [
                    end.cache.events - b.cache.events,
                    end.cache.token_bytes - b.cache.token_bytes,
                    end.cache.child_storage_nodes - b.cache.child_storage_nodes,
                    end.cache.nodes - b.cache.nodes,
                    end.cache.reused - b.cache.reused,
                ],
            },
            beginning,
            clean: end.diagnostics == b.diagnostics
                && end.checkpoint.diagnostics == b.checkpoint.diagnostics
                && end.stats.diagnostics_truncated == b.stats.diagnostics_truncated
                && end.stats.recovery_bytes == b.stats.recovery_bytes,
            balanced: end.checkpoint.open_markers == b.checkpoint.open_markers
                && end.checkpoint.rule_depth == b.checkpoint.rule_depth
                && end.checkpoint.nesting == b.checkpoint.nesting
                && !b.pending
                && !end.pending
                && b.stats.reused_node_count == end.stats.reused_node_count,
            peaks_raised: end.peaks != b.peaks,
        }
    }
}

#[derive(Default)]
struct Totals {
    replays: u64,
    shape_work: u64,
    charges: u64,
    rewinds: u64,
    node_ids: u64,
    token_ids: u64,
    cache_work: u64,
    output_events: u64,
    matched: u64,
    committed: u64,
    halted: u64,
    no_match: u64,
    clean: u64,
    provisional_candidates: u64,
    provisional_work: u64,
    candidate_bytes_max: u32,
    candidate_events_max: usize,
    warm_origin: u64,
    peaks_raised: u64,
    peak_budget_shortfall: u64,
    different_origin: u64,
    different_work: u64,
    different_effects: u64,
}

impl Totals {
    fn replay(&mut self, parser: &Parser<'_>, result: Attempt, previous: Outcome, replay: Outcome) {
        let replay_end = replay.effect.end;
        let start = replay.beginning.checkpoint;
        self.replays += 1;
        self.shape_work += replay.effect.work;
        self.charges += replay.effect.charges;
        self.rewinds += replay.effect.rewinds;
        self.node_ids += replay.effect.nodes;
        self.token_ids += replay.effect.tokens;
        self.cache_work += replay.effect.cache[0] + replay.effect.cache[1];
        self.output_events += replay_end.events.saturating_sub(start.events) as u64;
        if parser.is_halted() {
            self.halted += 1;
            return;
        }
        match result {
            Attempt::NoMatch => self.no_match += 1,
            Attempt::Committed => self.committed += 1,
            Attempt::Matched => self.matched += 1,
        }
        if result != Attempt::Matched || !previous.clean || !replay.clean {
            return;
        }
        self.clean += 1;
        self.warm_origin +=
            u64::from(previous.beginning.warm_origin || replay.beginning.warm_origin);
        self.peaks_raised += u64::from(previous.peaks_raised || replay.peaks_raised);
        if previous.beginning.checkpoint != start
            || previous.beginning.context != replay.beginning.context
        {
            self.different_origin += 1;
            return;
        }
        if previous.effect.work != replay.effect.work
            || previous.effect.charges != replay.effect.charges
        {
            self.different_work += 1;
            return;
        }
        if previous.effect != replay.effect {
            self.different_effects += 1;
            return;
        }
        if !previous.balanced
            || !replay.balanced
            || !previous.beginning.one_piece_batch
            || !replay.beginning.one_piece_batch
            || replay_end.cursor.offset <= start.cursor.offset
            || previous.beginning.warm_origin
            || replay.beginning.warm_origin
            || previous.peaks_raised
            || replay.peaks_raised
        {
            return;
        }
        let p = &parser.state;
        // The global historical peaks before either execution bound the
        // virtual live event depth of that execution. This is just a census
        // of available slack. It does not prove a recipe is safe.
        let event_headroom = p
            .peak_events
            .saturating_add(p.peak_open_markers)
            .saturating_add(3);
        if event_headroom > p.config.limits.max_events as usize {
            self.peak_budget_shortfall += 1;
            return;
        }
        self.provisional_candidates += 1;
        self.provisional_work += replay.effect.work;
        self.candidate_bytes_max = self
            .candidate_bytes_max
            .max((replay_end.cursor.offset - start.cursor.offset).0);
        self.candidate_events_max = self
            .candidate_events_max
            .max(replay_end.events.saturating_sub(start.events));
    }
}

#[derive(Default)]
struct Counters {
    replays: BTreeMap<Family, Totals>,
    choices: BTreeMap<Option<RuleId>, [u64; 6]>,
}

std::thread_local! {
    static CLOCK: RefCell<Clock> = RefCell::new(Clock::default());
    static COUNTERS: RefCell<Counters> = RefCell::new(Counters::default());
}

struct Trial {
    owner: Option<RuleId>,
    beginning: Snapshot,
    selected: Option<Outcome>,
}

struct Replay {
    depth: usize,
    family: Family,
    comparison: Option<Box<(Outcome, Snapshot)>>,
}

#[derive(Default)]
pub(super) struct State {
    trials: Vec<Trial>,
    replays: Vec<Replay>,
}

impl State {
    pub(super) fn resume(&self) {
        if TIME && !self.replays.is_empty() {
            CLOCK.with_borrow_mut(|clock| {
                clock.change(|active| {
                    active.extend(self.replays.iter().map(|entry| entry.family));
                })
            });
        }
    }

    pub(super) fn suspend(&self) {
        if TIME && !self.replays.is_empty() {
            CLOCK.with_borrow_mut(|clock| {
                clock.change(|active| {
                    active.truncate(active.len() - self.replays.len());
                })
            });
        }
    }

    pub(super) fn begin_choice(
        &mut self,
        parser: &Parser<'_>,
        state: GrammarState,
        work: u64,
        input_final: bool,
    ) {
        if TIME {
            return;
        }
        self.trials.push(Trial {
            owner: parser.current_rule(),
            beginning: Snapshot::capture(parser, state, work, input_final),
            selected: None,
        });
    }

    pub(super) fn begin_trial(
        &mut self,
        parser: &Parser<'_>,
        state: GrammarState,
        work: u64,
        input_final: bool,
    ) {
        if !TIME {
            self.trials.last_mut().expect("active choice").beginning =
                Snapshot::capture(parser, state, work, input_final);
        }
    }

    pub(super) fn trial(
        &mut self,
        parser: &Parser<'_>,
        state: GrammarState,
        work: u64,
        input_final: bool,
        result: Attempt,
        wins: bool,
    ) {
        if TIME {
            return;
        }
        let trial = self.trials.last_mut().expect("active choice");
        let slot = if parser.is_halted() {
            0
        } else {
            match result {
                Attempt::NoMatch => 1,
                Attempt::Committed => 2,
                Attempt::Matched => 3,
            }
        };
        COUNTERS.with_borrow_mut(|c| c.choices.entry(trial.owner).or_default()[slot] += 1);
        if wins {
            trial.selected = Some(Outcome::new(
                trial.beginning,
                Snapshot::capture(parser, state, work, input_final),
            ));
        }
    }

    pub(super) fn stop_choice(&mut self) {
        if !TIME {
            let trial = self.trials.pop().expect("active choice");
            COUNTERS.with_borrow_mut(|c| c.choices.entry(trial.owner).or_default()[4] += 1);
        }
    }

    pub(super) fn replay(
        &mut self,
        depth: usize,
        expression: &GrammarExpression,
        parser: &Parser<'_>,
        state: GrammarState,
        work: u64,
        input_final: bool,
    ) {
        let family = Family::new(parser, expression);
        let comparison = if TIME {
            CLOCK.with_borrow_mut(|clock| clock.change(|active| active.push(family)));
            None
        } else {
            let trial = self.trials.pop().expect("active choice");
            COUNTERS.with_borrow_mut(|c| c.choices.entry(trial.owner).or_default()[5] += 1);
            Some(Box::new((
                trial.selected.expect("winning trial recorded"),
                Snapshot::capture(parser, state, work, input_final),
            )))
        };
        self.replays.push(Replay {
            depth,
            family,
            comparison,
        });
    }

    pub(super) fn finish_ready(
        &mut self,
        depth: usize,
        parser: &Parser<'_>,
        state: GrammarState,
        work: u64,
        input_final: bool,
        result: Attempt,
    ) {
        while self
            .replays
            .last()
            .is_some_and(|replay| replay.depth == depth)
        {
            let replay = self.replays.pop().expect("ready replay");
            if TIME {
                CLOCK.with_borrow_mut(|clock| {
                    clock.change(|active| {
                        assert!(active.pop() == Some(replay.family));
                    })
                });
            } else {
                let (winner, beginning) = *replay.comparison.expect("census replay");
                let final_outcome = Outcome::new(
                    beginning,
                    Snapshot::capture(parser, state, work, input_final),
                );
                COUNTERS.with_borrow_mut(|c| {
                    c.replays.entry(replay.family).or_default().replay(
                        parser,
                        result,
                        winner,
                        final_outcome,
                    )
                });
            }
        }
    }
}

pub(super) fn begin() {
    if TIME {
        let now = Instant::now();
        CLOCK.with_borrow_mut(|c| {
            *c = Clock {
                begin: Some(now),
                previous: Some(now),
                ..Clock::default()
            }
        });
    } else {
        COUNTERS.with_borrow_mut(|c| *c = Counters::default());
    }
}

pub(super) fn report(parser: &Parser<'_>, steps: u64, child_work: u64, peak_frames: usize) {
    if TIME {
        CLOCK.with_borrow_mut(|c| {
            assert!(c.active.is_empty());
            c.split();
            let elapsed = c.begin.take().expect("timed grammar").elapsed().as_nanos() as u64;
            c.previous = None;
            eprintln!("best-choice-clock {} {}", elapsed, c.outside);
            for (&family, t) in &c.times {
                let (owner, alternative) = family.labels();
                eprintln!(
                    "best-choice-time {} {} {} {} {}",
                    owner, alternative, t.exclusive, t.outermost, t.inclusive_union
                );
            }
        });
    } else {
        COUNTERS.with_borrow(|c| {
            for (&family, t) in &c.replays {
                let (owner, alternative) = family.labels();
                let row = [
                    t.replays,
                    t.shape_work,
                    t.charges,
                    t.rewinds,
                    t.node_ids,
                    t.token_ids,
                    t.cache_work,
                    t.output_events,
                    t.matched,
                    t.committed,
                    t.halted,
                    t.no_match,
                    t.clean,
                    t.provisional_candidates,
                    t.provisional_work,
                    u64::from(t.candidate_bytes_max),
                    t.candidate_events_max as u64,
                    t.warm_origin,
                    t.peaks_raised,
                    t.peak_budget_shortfall,
                    t.different_origin,
                    t.different_work,
                    t.different_effects,
                ];
                eprintln!(
                    "best-choice-work {} {} {}",
                    owner,
                    alternative,
                    row.iter().map(u64::to_string).collect::<Vec<_>>().join(" ")
                );
            }
            for (&owner, counts) in &c.choices {
                eprintln!(
                    "best-choice-trials {} {}",
                    owner
                        .and_then(canonical_rule_name)
                        .unwrap_or("anonymous-choice"),
                    counts
                        .iter()
                        .map(u64::to_string)
                        .collect::<Vec<_>>()
                        .join(" ")
                );
            }
        });
        let p = &parser.state;
        eprintln!(
            "best-choice-root {} {} {} {} {} {} {} {} {} {} {}",
            steps,
            child_work,
            peak_frames,
            p.fuel,
            p.rewinds,
            p.rewound_bytes,
            p.peak_events,
            p.peak_open_markers,
            p.rules.peak_depth(),
            parser.ids.next_node(),
            parser.ids.next_token()
        );
    }
}
