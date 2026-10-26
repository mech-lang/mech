//! Own/check generated continuation boundaries; never evaluate a Mech state.
//! Snapshot and captured-input owners share these checked continuation
//! boundaries. Source acquisition and final admission belong to the session.
use super::super::{NativeExecutionPhase, program_abi};
use super::{
    Invocation, NativeProgramDescriptor, NativeProgramExecutionError as Error, amount, invalid,
};
use mech_engine::native_artifact::{
    NativeLibraryWorkBudget, NativeMatchValue, NativeTextLineWindow,
};

pub(super) fn validate_window(
    descriptor: &NativeProgramDescriptor,
    input: Option<Input>,
    window: &NativeTextLineWindow<'_>,
    work: &NativeLibraryWorkBudget,
) -> Result<(), Error> {
    let input = input.ok_or_else(|| invalid("line window outside continuation entry"))?;
    let region = descriptor
        .metadata()
        .prefix
        .as_ref()
        .ok_or_else(|| invalid("line window lacks exact scan proof"))?;
    let source = region.frontier.read().count().lines();
    if descriptor.metadata().borrowed_strings.regions[region.borrowed]
        .proof
        .matrices()
        != [source.matrix()]
        || window.available_lines() != input.available
        || window.final_facts().is_some() != input.eof
    {
        return Err(invalid(
            "line window differs from exact completed source prefix",
        ));
    }
    let site = source.producer() as usize;
    descriptor.metadata().calls[site]
        .as_ref()
        .ok_or_else(|| invalid("line window producer absent"))?
        .bind_text_line_window(window, work)
        .map_err(|error| Error::Library {
            origin: descriptor.metadata().plan.blocks()[site].origin(),
            error,
        })?;
    Ok(())
}

impl<'state, 'plan, 'visit> Invocation<'state, 'visit, 'plan> {
    pub(super) fn prefix_line_window(
        &self,
        matrix: NativeMatchValue,
    ) -> Option<&'visit NativeTextLineWindow<'visit>> {
        let region = self.descriptor.metadata().prefix.as_ref()?;
        (region.frontier.read().count().lines().matrix() == matrix)
            .then_some(self.line_window)
            .flatten()
    }
}

#[derive(Clone, Copy)]
pub(super) struct Input {
    pub resume_site: u32,
    pub available: u64,
    pub eof: bool,
}

/// Coordinates reported by generated instructions, not a decoded Mech state.
/// A sealed range is immutable; an open lower bound only moves forward.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Retention {
    pub first: u64,
    pub end: Option<u64>,
}
impl Retention {
    fn checked(args: [u64; 3], available: u64, prior: Option<Self>) -> Result<Self, Error> {
        let [first, end, closed] = args;
        let extent = available
            .checked_add(1)
            .ok_or_else(|| invalid("prefix extent overflows"))?;
        if first == 0
            || closed > 1
            || first > extent
            || (closed == 0 && end != 0)
            || (closed == 1 && (end < first || end > extent))
        {
            return Err(invalid(
                "native retention coordinates exceed completed input",
            ));
        }
        let retained = Self {
            first,
            end: (closed == 1).then_some(end),
        };
        if let Some(prior) = prior {
            if prior.end.is_some() && retained != prior {
                return Err(invalid(
                    "native retention reopened or changed a sealed range",
                ));
            }
            if retained.end != Some(first) && first < prior.first {
                return Err(invalid("native retention revived a discarded prefix"));
            }
        }
        Ok(retained)
    }
}
pub(super) struct State {
    pub input: Input,
    pub paused: Option<u32>,
    pub scanned: Option<u32>,
    pub retained: Option<Retention>,
    previous_retention: Option<Retention>,
    borrowed_resumed: bool,
    tail_resumed: bool,
}
impl State {
    pub fn new(input: Input, previous_retention: Option<Retention>) -> Self {
        Self {
            input,
            paused: None,
            scanned: None,
            retained: None,
            previous_retention,
            borrowed_resumed: false,
            tail_resumed: false,
        }
    }
}

pub(super) fn checkpoint_words(descriptor: &NativeProgramDescriptor) -> Result<usize, Error> {
    let region = descriptor
        .metadata()
        .prefix
        .as_ref()
        .ok_or_else(|| invalid("prefix lacks same-plan proof"))?;
    let borrowed = &descriptor.metadata().borrowed_strings.regions[region.borrowed];
    (borrowed.end - borrowed.ready)
        .checked_add(3)
        .and_then(|words| words.checked_add(descriptor.metadata().max_library_inputs.max(1)))
        .ok_or_else(|| invalid("prefix checkpoint lane size overflows"))
}

pub(super) fn check_cleared(
    descriptor: &NativeProgramDescriptor,
    locals: &[u64],
    arguments: &[u64],
) -> Result<(), Error> {
    let region = descriptor
        .metadata()
        .prefix
        .as_ref()
        .ok_or_else(|| invalid("prefix lacks same-plan proof"))?;
    let borrowed = &descriptor.metadata().borrowed_strings.regions[region.borrowed];
    if locals[borrowed.ready..borrowed.end]
        .iter()
        .any(|word| *word != 0)
        || arguments.iter().any(|word| *word != 0)
        || descriptor.metadata().closed_tails.sites[region.tail.header() as usize].is_some_and(
            |closed| {
                let words = descriptor.metadata().closed_tails.regions[closed].words;
                locals[words..words + 3].iter().any(|word| *word != 0)
            },
        )
    {
        return Err(invalid("native prefix checkpoint retained borrowed words"));
    }
    Ok(())
}
impl Invocation<'_, '_, '_> {
    pub(super) fn validate_prefix_extent(
        &self,
        matrix: NativeMatchValue,
        actual: usize,
    ) -> Result<(), Error> {
        let Some(prefix) = &self.prefix else {
            return Ok(());
        };
        let region = self
            .descriptor
            .metadata()
            .prefix
            .as_ref()
            .ok_or_else(|| invalid("prefix lacks same-plan proof"))?;
        if region.frontier.read().count().lines().matrix() == matrix
            && (prefix.input.available > actual as u64
                || (prefix.input.eof && prefix.input.available != actual as u64))
        {
            return Err(invalid(
                "prefix availability differs from genuine source extent",
            ));
        }
        Ok(())
    }
    pub(super) fn check_prefix_header(&self, site: u32) -> Result<(), Error> {
        let region = self
            .descriptor
            .metadata()
            .prefix
            .as_ref()
            .ok_or_else(|| invalid("prefix lacks same-plan proof"))?;
        let prefix = self
            .prefix
            .as_ref()
            .ok_or_else(|| invalid("prefix helper outside continuation phase"))?;
        if site != region.tail.header()
            || self.activation_phase
            || prefix.paused.is_some()
            || prefix.scanned.is_some()
            || self.state.control.completed
            || self.state.output_cursor != 0
            || self.state.consumed
            || self.state.control.current != Some(region.frontier.iteration())
            || self
                .progress
                .chunks_exact(2)
                .enumerate()
                .any(|(iteration, state)| {
                    if iteration == region.frontier.iteration() as usize {
                        state != [0, 1]
                    } else {
                        state[1] != 0
                    }
                })
            || ![Some(region.tail.initialize()), Some(region.tail.latch())]
                .contains(&self.state.control.previous)
        {
            return Err(invalid("prefix outside its exact active header boundary"));
        }
        Ok(())
    }
    /// The original Tail finish has run, but its exit and every consumer after
    /// it have not. No caller-provided state ordinal is interpreted here.
    pub(super) fn check_prefix_exit(&self, site: u32) -> Result<(), Error> {
        let region = self
            .descriptor
            .metadata()
            .prefix
            .as_ref()
            .ok_or_else(|| invalid("scan exit lacks same-plan proof"))?;
        let prefix = self
            .prefix
            .as_ref()
            .ok_or_else(|| invalid("scan exit outside continuation phase"))?;
        if site != region.tail.exit()
            || !prefix.input.eof
            || self.activation_phase
            || prefix.paused.is_some()
            || self.state.control.completed
            || self.state.output_cursor != 0
            || self.state.consumed
            || self.state.control.current != Some(region.frontier.iteration())
            || self.state.control.previous != Some(region.tail.finish())
            || self
                .progress
                .chunks_exact(2)
                .enumerate()
                .any(|(iteration, state)| {
                    if iteration == region.frontier.iteration() as usize {
                        state != [0, 1]
                    } else {
                        state[1] != 0
                    }
                })
        {
            return Err(invalid("scan outside its exact unvisited exit boundary"));
        }
        Ok(())
    }
    pub(super) fn prefix_helper(
        &mut self,
        site: u64,
        operation: u64,
        args: [u64; 3],
    ) -> Result<Option<u64>, Error> {
        let site = u32::try_from(site).map_err(|_| invalid("prefix site overflows"))?;
        // Every resume/pause guard scans the progress table. Prepay EACH scan,
        // not just the single owning entry's epilogue, on this same account.
        self.state.control.work.charge(
            NativeExecutionPhase::Setup,
            amount(self.progress.len() / 2, 64, 1024)?,
        )?;
        if operation == program_abi::PREFIX_SCANNED {
            self.check_prefix_exit(site)?;
        } else {
            self.check_prefix_header(site)?;
        }
        if (operation != program_abi::PREFIX_PAUSE && args != [0; 3])
            || self.window_region.is_some()
            || self.selected_join.is_some()
        {
            return Err(invalid("malformed or overlapping prefix loan"));
        }
        let region = self.descriptor.metadata().prefix.as_ref().unwrap();
        let borrowed = region.borrowed;
        let input = self.prefix.as_ref().unwrap().input;
        match operation {
            program_abi::PREFIX_SCANNED => {
                if self.prefix.as_ref().unwrap().scanned.is_some()
                    || self.closed_tail.is_some()
                    || !self
                        .borrowed_strings
                        .as_ref()
                        .is_some_and(|state| state.region == borrowed)
                {
                    return Err(invalid("scan exit omitted its exact loan reconciliation"));
                }
                self.state.control.work.charge(
                    NativeExecutionPhase::Setup,
                    amount(self.argument_capacity, 16, 1024)?,
                )?;
                for word in 0..self.argument_capacity {
                    self.write_borrowed_word(word, 0)?;
                }
                self.borrowed_strings = None;
                self.prefix.as_mut().unwrap().scanned = Some(site);
                Ok(None)
            }
            program_abi::PREFIX_PAUSE => {
                let retained = Retention::checked(
                    args,
                    input.available,
                    self.prefix.as_ref().unwrap().previous_retention,
                )?;
                if input.eof
                    || self.closed_tail.is_some()
                    || !self
                        .borrowed_strings
                        .as_ref()
                        .is_some_and(|state| state.region == borrowed)
                {
                    return Err(invalid(
                        "prefix pause omitted its exact loan reconciliation",
                    ));
                }
                self.state.control.work.charge(
                    NativeExecutionPhase::Setup,
                    amount(self.argument_capacity, 16, 1024)?,
                )?;
                for word in 0..self.argument_capacity {
                    self.write_borrowed_word(word, 0)?;
                }
                self.borrowed_strings = None;
                self.prefix.as_mut().unwrap().retained = Some(retained);
                self.prefix.as_mut().unwrap().paused = Some(site);
                Ok(None)
            }
            program_abi::PREFIX_BORROWED_RESUME => {
                let prefix = self.prefix.as_ref().unwrap();
                if input.resume_site != site || prefix.borrowed_resumed || prefix.tail_resumed {
                    return Err(invalid("duplicate or wrong-site borrowed prefix resume"));
                }
                let result = self.prepare_borrowed_region(site as usize, args, borrowed)?;
                self.prefix.as_mut().unwrap().borrowed_resumed = true;
                Ok(result)
            }
            program_abi::PREFIX_TAIL_RESUME => {
                let prefix = self.prefix.as_ref().unwrap();
                if input.resume_site != site || !prefix.borrowed_resumed || prefix.tail_resumed {
                    return Err(invalid("duplicate or unordered prefix fuel resume"));
                }
                let closed = self.descriptor.metadata().closed_tails.sites[site as usize]
                    .ok_or_else(|| invalid("prefix fuel resume outside closed Tail"))?;
                let result = self.begin_closed_region(site as usize, args, closed)?;
                self.prefix.as_mut().unwrap().tail_resumed = true;
                Ok(result)
            }
            _ => Err(invalid("unknown native prefix helper")),
        }
    }
}

#[cfg(test)]
mod retention_tests {
    use super::Retention;

    #[test]
    fn coordinates_reject_unknown_extents_regressions_and_sealed_range_mutation() {
        let open = Retention::checked([3, 0, 0], 3, None).unwrap();
        assert_eq!(
            open,
            Retention {
                first: 3,
                end: None
            }
        );
        for args in [
            [0, 0, 0],
            [5, 0, 0],
            [3, 1, 0],
            [3, 0, 2],
            [3, 2, 1],
            [3, 5, 1],
            [2, 0, 0],
            [2, 4, 1],
        ] {
            assert!(Retention::checked(args, 3, Some(open)).is_err(), "{args:?}");
        }
        let sealed = Retention::checked([3, 4, 1], 3, Some(open)).unwrap();
        assert_eq!(
            Retention::checked([3, 4, 1], 100, Some(sealed)).unwrap(),
            sealed
        );
        for args in [[3, 0, 0], [2, 4, 1], [3, 5, 1], [1, 1, 1]] {
            assert!(
                Retention::checked(args, 100, Some(sealed)).is_err(),
                "{args:?}"
            );
        }
        let empty = Retention::checked([1, 1, 1], 3, Some(open)).unwrap();
        assert_eq!(
            empty,
            Retention {
                first: 1,
                end: Some(1)
            }
        );
        assert!(Retention::checked([1, 2, 1], 100, Some(empty)).is_err());
        assert!(Retention::checked([1, 0, 0], u64::MAX, None).is_err());
    }
}
