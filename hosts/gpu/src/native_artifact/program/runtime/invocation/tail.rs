//! Reconcile a closed generated Tail to the SAME invocation account.
//! The generated body consumes finite native fuel; this never executes a loop.
use crate::native_artifact::NativeExecutionPhase;
use crate::native_artifact::program::prepare::numeric_plan::ExitKind;
use crate::native_artifact::program::runtime::invocation::Invocation;
use crate::native_artifact::program::runtime::{
    NativeProgramExecutionError as Error, amount, invalid,
};

#[derive(Clone, Copy)]
pub(super) enum Region {
    Tail(usize),
    Segment(usize),
}
pub(in crate::native_artifact::program::runtime) struct State {
    pub(super) region: Region,
    pub(super) used: (u64, u64),
    pub(super) remaining: (u64, u64),
    pub(super) entry: u32,
    pub(super) partial: bool,
}
impl Invocation<'_, '_, '_> {
    pub(in crate::native_artifact::program::runtime) fn begin_closed_tail(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let layout = &self.descriptor.metadata().closed_tails;
        let region = layout
            .initializers
            .get(site)
            .copied()
            .flatten()
            .filter(|_| self.state.control.previous == Some(site as u32))
            .or_else(|| {
                layout.sites.get(site).copied().flatten().filter(|&id| {
                    let region = &layout.regions[id];
                    region.tail.header() as usize == site
                        && region
                            .numeric
                            .as_ref()
                            .is_some_and(|proof| !proof.exits.is_empty())
                        && self.state.control.previous == Some(region.tail.latch())
                        && self.numeric_prepaid.is_none()
                })
            })
            .ok_or_else(|| {
                invalid("closed Tail outside initializer or cold continuation header")
            })?;
        self.begin_closed_region(site, args, region)
    }
    pub(in crate::native_artifact::program::runtime) fn begin_closed_region(
        &mut self,
        _site: usize,
        args: [u64; 3],
        region: usize,
    ) -> Result<Option<u64>, Error> {
        let proof = &self.descriptor.metadata().closed_tails.regions[region];
        if args != [0; 3]
            || self.closed_tail.is_some()
            || self.window_region.is_some()
            || (self.state.consumed && proof.map.is_none() && proof.numeric.is_none())
        {
            return Err(invalid("duplicate or malformed closed Tail entry"));
        }
        let layout = &self.descriptor.metadata().closed_tails;
        let proof = &layout.regions[region];
        if self.numeric_prepaid.is_some()
            && proof
                .numeric
                .as_ref()
                .is_some_and(|proof| !proof.exits.is_empty())
        {
            return Ok(Some(0));
        }
        if self.state.control.current != Some(proof.iteration)
            || self.progress[proof.iteration as usize * 2..][..2] != [0, 1]
        {
            return Err(invalid("closed Tail outside active invocation"));
        }
        if proof.numeric.is_some()
            && proof
                .tail
                .parent()
                .is_some_and(|parent| self.progress[parent as usize * 2 + 1] != 1)
        {
            return Err(invalid("numeric child has no suspended active parent"));
        }
        // Covers entry/exit metadata and all exceptional cleanup dispatch arms.
        self.state.control.work.charge(
            NativeExecutionPhase::Setup,
            amount(layout.regions.len(), 128, 1024)?,
        )?;
        if proof.borrowed.is_some_and(|region| {
            !self
                .borrowed_strings
                .as_ref()
                .is_some_and(|state| state.active && state.region == region)
        }) {
            return Ok(Some(0));
        }
        let Some(mut remaining) = self.state.library.unscoped_remaining() else {
            return Ok(Some(0));
        };
        if proof.numeric.is_some() {
            if !self.begin_numeric_views(region)? {
                return Ok(Some(0));
            }
            // Entry validation spent on this SAME ledger. Fund the native body
            // from the post-entry remainder, never the earlier preview.
            remaining = self
                .state
                .library
                .unscoped_remaining()
                .ok_or_else(|| invalid("numeric entry changed account scope"))?;
        }
        if proof.map.is_some() {
            let Some(words) = self.begin_closed_map(region)? else {
                return Ok(Some(0));
            };
            for (index, word) in words.into_iter().enumerate() {
                self.write_borrowed_word(index, word)?;
            }
        } else {
            self.write_borrowed_word(0, remaining.0)?;
            self.write_borrowed_word(1, remaining.1)?;
        }
        self.closed_tail = Some(State {
            region: Region::Tail(region),
            used: self.state.library.used(),
            remaining,
            entry: self
                .state
                .control
                .previous
                .ok_or_else(|| invalid("closed Tail has no current predecessor"))?,
            partial: false,
        });
        Ok(Some(1))
    }

    pub(in crate::native_artifact::program::runtime) fn end_closed_tail(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let state = self
            .closed_tail
            .take()
            .ok_or_else(|| invalid("closed Tail exit without entry"))?;
        let descriptor = self.descriptor;
        let layout = &descriptor.metadata().closed_tails;
        let Region::Tail(region) = state.region else {
            return Err(invalid("closed Tail exit received a parent segment loan"));
        };
        let proof = &layout.regions[region];
        if self.state.control.previous != Some(state.entry)
            || self.state.control.current != Some(proof.iteration)
            || self.progress[proof.iteration as usize * 2..][..2] != [0, 1]
            || (proof.map.is_none()
                && (self.state.library.used() != state.used
                    || self.state.library.unscoped_remaining() != Some(state.remaining)))
        {
            return Err(invalid(
                "closed Tail account or entry changed during native execution",
            ));
        }
        if proof.numeric.is_some()
            && proof
                .tail
                .parent()
                .is_some_and(|parent| self.progress[parent as usize * 2 + 1] != 1)
        {
            return Err(invalid("numeric child's suspended parent changed"));
        }
        let (compute, comparison, traversals) = if proof.map.is_some() {
            if args[1] != 0 {
                return Err(invalid("closed map comparison lane changed"));
            }
            (0, 0, self.reconcile_closed(region, args[0], args[2])?)
        } else {
            let compute = state
                .remaining
                .0
                .checked_sub(args[0])
                .ok_or_else(|| invalid("closed Tail compute fuel was renewed"))?;
            let comparison = state
                .remaining
                .1
                .checked_sub(args[1])
                .ok_or_else(|| invalid("closed Tail comparison fuel was renewed"))?;
            let traversals =
                if let Some(phase) = proof.numeric.as_ref().and_then(|p| p.phase.as_ref()) {
                    let mut expected = (0u64, 0u64);
                    let mut root = 0;
                    for (index, debit) in phase.loops.iter().enumerate() {
                        let count = self.read_borrowed_word(2 + debit.counter)?;
                        if index == 0 {
                            root = count;
                        }
                        expected.0 = count
                            .checked_mul(debit.compute)
                            .and_then(|n| expected.0.checked_add(n))
                            .ok_or_else(|| invalid("phase compute settlement overflows"))?;
                        expected.1 = count
                            .checked_mul(debit.comparison)
                            .and_then(|n| expected.1.checked_add(n))
                            .ok_or_else(|| invalid("phase comparison settlement overflows"))?;
                    }
                    if expected != (compute, comparison) {
                        return Err(invalid("phase fuel differs from actual traversal debits"));
                    }
                    root
                } else {
                    let traversals = compute / proof.compute;
                    if compute % proof.compute != 0
                        || traversals.checked_mul(proof.comparison) != Some(comparison)
                    {
                        return Err(invalid(
                            "closed Tail fuel differs from proved body envelope",
                        ));
                    }
                    traversals
                };
            (compute, comparison, traversals)
        };
        let previous = match args[2] {
            0 if site == proof.tail.exit() as usize && (traversals > 0 || state.partial) => proof.tail.finish(),
            // No fuel for a complete body: reconcile before the ordinary header
            // admits any further instruction. No allowance is refilled.
            1 if site == proof.tail.header() as usize => {
                if traversals == 0 && !state.partial {
                    state.entry
                } else {
                    proof.tail.latch()
                }
            }
            4 if site == proof.tail.header() as usize && self.prefix.is_some() => {
                if traversals == 0 {
                    state.entry
                } else {
                    proof.tail.latch()
                }
            }
            2 | 3 if layout.sites.get(site).copied().flatten() == Some(region) => site as u32,
            5 if proof.numeric.as_ref().and_then(|p| p.action(site)).is_some_and(|a| matches!(a,
                crate::native_artifact::program::prepare::readonly_numeric::Operation::Write { .. }
                | crate::native_artifact::program::prepare::readonly_numeric::Operation::Append { .. })) => site as u32,
            6 if (traversals > 0 || state.partial) && self.numeric_prepaid.is_none() => proof.numeric.as_ref()
                .and_then(|proof| proof.exits.iter().find(|exit| exit.to == site && matches!(exit.kind, ExitKind::Unsupported)))
                .map(|exit| exit.from as u32)
                .ok_or_else(|| invalid("numeric cold exit is outside its exact source edge"))?,
            7 if (traversals > 0 || state.partial) && self.numeric_prepaid.is_none() => proof.numeric.as_ref()
                .and_then(|proof| proof.exits.iter().find(|exit| exit.to == site && !matches!(exit.kind, ExitKind::Unsupported)))
                .map(|exit| exit.from as u32)
                .ok_or_else(|| invalid("numeric fuel exit is outside its exact source edge"))?,
            _ => return Err(invalid("closed Tail exit outside its proved boundary")),
        };
        self.synchronize_numeric_carrier(region, site, args[2])?;
        self.state
            .library
            .charge(compute, comparison)
            .map_err(|error| Error::Collection {
                origin: descriptor.metadata().plan.blocks()[site].origin(),
                error,
            })?;
        // This is one proved bulk transition, not fabricated/replayed per-op
        // visits. The report counts the remaining actual admitted CFG visits.
        self.state.control.previous = Some(previous);
        if matches!(args[2], 6 | 7) {
            let exit = proof
                .numeric
                .as_ref()
                .unwrap()
                .exits
                .iter()
                .find(|exit| exit.to == site)
                .unwrap();
            let (active, deepest) = match exit.kind {
                ExitKind::Unsupported if args[2] == 6 => {
                    let active = descriptor.metadata().plan.blocks()[exit.from]
                        .iteration()
                        .ok_or_else(|| invalid("cold continuation lacks its active Tail"))?;
                    (active, active)
                }
                ExitKind::ChildInitialize { iteration } | ExitKind::ChildHeader { iteration }
                    if args[2] == 7 =>
                {
                    let parent = descriptor.metadata().plan.iterations()[iteration as usize]
                        .as_ref()
                        .parent()
                        .ok_or_else(|| invalid("fuel continuation lacks its paid parent"))?;
                    let active = if matches!(exit.kind, ExitKind::ChildHeader { .. }) {
                        iteration
                    } else {
                        parent
                    };
                    (active, parent)
                }
                _ => return Err(invalid("numeric continuation differs from its exit kind")),
            };
            // Native synchronous Tail children have no hidden counted-loop or
            // collection state. Export/finalization and all fallible charges
            // finish before restoring their exact lexical activation chain.
            for child in proof
                .numeric
                .as_ref()
                .unwrap()
                .phase
                .as_ref()
                .unwrap()
                .loops
                .iter()
                .skip(1)
            {
                if self.progress[child.iteration as usize * 2..][..2] != [0, 0] {
                    return Err(invalid("fuel continuation found an active ordinary child"));
                }
            }
            let mut at = active;
            loop {
                self.progress[at as usize * 2..][..2].copy_from_slice(&[0, 1]);
                if at == proof.iteration {
                    break;
                }
                at = descriptor.metadata().plan.iterations()[at as usize]
                    .as_ref()
                    .parent()
                    .ok_or_else(|| invalid("fuel continuation escapes its native root"))?;
            }
            self.state.control.current = Some(active);
            self.numeric_prepaid = Some(super::NumericPrepaid { region, deepest });
            self.state.scalar_path = None;
        }
        if proof.map.is_some() && args[2] == 0 {
            self.state.map_owners.closed_exit = Some((previous, proof.tail.exit()));
        }
        self.state.consumed = proof.numeric.is_some() && !matches!(args[2], 2 | 5);
        if args[2] == 3 {
            // Arithmetic/assertion/rejection exits have no leaf callback to consume the
            // owner loan. Drop it before the existing typed error is returned.
            self.borrowed_strings = None;
        }
        Ok(None)
    }

    /// Continue the already prepaid traversal after its one-time fallible
    /// materialization. The new loan is the SAME post-materialization account;
    /// no source header/body allowance is renewed or charged twice.
    pub(super) fn resume_numeric_after_write(
        &mut self,
        region: usize,
        site: usize,
    ) -> Result<(), Error> {
        if self.closed_tail.is_some() || self.state.control.previous != Some(site as u32) {
            return Err(invalid(
                "numeric materialization resume outside settled site",
            ));
        }
        let remaining = self
            .state
            .library
            .unscoped_remaining()
            .ok_or_else(|| invalid("numeric materialization changed account scope"))?;
        self.write_borrowed_word(0, remaining.0)?;
        self.write_borrowed_word(1, remaining.1)?;
        if let Some(phase) = self.descriptor.metadata().closed_tails.regions[region]
            .numeric
            .as_ref()
            .and_then(|p| p.phase.as_ref())
        {
            for debit in &phase.loops {
                self.write_borrowed_word(2 + debit.counter, 0)?;
            }
        }
        self.closed_tail = Some(State {
            region: Region::Tail(region),
            used: self.state.library.used(),
            remaining,
            entry: site as u32,
            partial: true,
        });
        Ok(())
    }
}
