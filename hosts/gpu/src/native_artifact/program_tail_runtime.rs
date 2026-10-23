//! Reconcile a closed generated Tail to the SAME invocation account.
//! The generated body consumes finite native fuel; this never executes a loop.
use super::super::NativeExecutionPhase;
use super::{Context, NativeProgramExecutionError as Error, amount, invalid};

pub(super) struct State {
    region: usize,
    used: (u64, u64),
    remaining: (u64, u64),
    entry: u32,
}
impl Context<'_, '_> {
    pub(super) fn begin_closed_tail(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let layout = &self.control.descriptor.closed_tails;
        let region = layout
            .initializers
            .get(site)
            .copied()
            .flatten()
            .ok_or_else(|| invalid("closed Tail outside initializer"))?;
        self.begin_closed_region(site, args, region)
    }
    pub(super) fn begin_closed_region(
        &mut self,
        _site: usize,
        args: [u64; 3],
        region: usize,
    ) -> Result<Option<u64>, Error> {
        let proof = &self.control.descriptor.closed_tails.regions[region];
        if args != [0; 3]
            || self.closed_tail.is_some()
            || self.window_region.is_some()
            || (self.consumed && proof.map.is_none())
        {
            return Err(invalid("duplicate or malformed closed Tail entry"));
        }
        let layout = &self.control.descriptor.closed_tails;
        let proof = &layout.regions[region];
        if self.control.current != Some(proof.iteration)
            || self.control.progress[proof.iteration as usize * 2..][..2] != [0, 1]
        {
            return Err(invalid("closed Tail outside active invocation"));
        }
        // Covers entry/exit metadata and all exceptional cleanup dispatch arms.
        self.control.work.charge(
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
        let Some(remaining) = self.library.unscoped_remaining() else {
            return Ok(Some(0));
        };
        if proof.map.is_some() {
            let Some(words) = self.begin_closed_map(region)? else { return Ok(Some(0)); };
            for (index, word) in words.into_iter().enumerate() { self.write_borrowed_word(index, word)?; }
        } else {
            self.write_borrowed_word(0, remaining.0)?;
            self.write_borrowed_word(1, remaining.1)?;
        }
        self.closed_tail = Some(State {
            region,
            used: self.library.used(),
            remaining,
            entry: self
                .control
                .previous
                .ok_or_else(|| invalid("closed Tail has no current predecessor"))?,
        });
        Ok(Some(1))
    }

    pub(super) fn end_closed_tail(
        &mut self,
        site: usize,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let state = self
            .closed_tail
            .take()
            .ok_or_else(|| invalid("closed Tail exit without entry"))?;
        let descriptor = self.control.descriptor;
        let layout = &descriptor.closed_tails;
        let proof = &layout.regions[state.region];
        if self.control.previous != Some(state.entry)
            || self.control.current != Some(proof.iteration)
            || self.control.progress[proof.iteration as usize * 2..][..2] != [0, 1]
            || (proof.map.is_none() && (self.library.used() != state.used
                || self.library.unscoped_remaining() != Some(state.remaining)))
        {
            return Err(invalid(
                "closed Tail account or entry changed during native execution",
            ));
        }
        let (compute, comparison, traversals) = if proof.map.is_some() {
            if args[1] != 0 { return Err(invalid("closed map comparison lane changed")); }
            (0, 0, self.end_closed_map(state.region, args[0])?)
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
            let traversals = compute / proof.compute;
            if compute % proof.compute != 0
                || traversals.checked_mul(proof.comparison) != Some(comparison)
            {
                return Err(invalid(
                    "closed Tail fuel differs from proved body envelope",
                ));
            }
            (compute, comparison, traversals)
        };
        let previous = match args[2] {
            0 if site == proof.tail.exit() as usize && traversals > 0 => proof.tail.finish(),
            // No fuel for a complete body: reconcile before the ordinary header
            // admits any further instruction. No allowance is refilled.
            1 if site == proof.tail.header() as usize => {
                if traversals == 0 {
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
            2 | 3 if layout.sites.get(site).copied().flatten() == Some(state.region) => site as u32,
            _ => return Err(invalid("closed Tail exit outside its proved boundary")),
        };
        self.library
            .charge(compute, comparison)
            .map_err(|error| Error::Collection {
                origin: descriptor.plan.blocks()[site].origin(),
                error,
            })?;
        // This is one proved bulk transition, not fabricated/replayed per-op
        // visits. The report counts the remaining actual admitted CFG visits.
        self.control.previous = Some(previous);
        if proof.map.is_some() && args[2] == 0 {
            self.map_owners.closed_exit = Some((previous, proof.tail.exit()));
        }
        self.consumed = false;
        if args[2] == 3 {
            // Arithmetic/rejection exits have no leaf callback to consume the
            // owner loan. Drop it before the existing typed error is returned.
            self.borrowed_strings = None;
        }
        Ok(None)
    }
}
