//! Own/check generated continuation boundaries; never evaluate a Mech state.
//! The initial qualification uses genuine FULL snapshots. Incremental source
//! storage and deferred source admissions are separate, not enabled here.
use super::super::{NativeExecutionPhase, program_abi};
use super::{
    Context, NativeProgramDescriptor, NativeProgramExecutionError as Error, amount, invalid,
};
use mech_engine::native_artifact::NativeMatchValue;

#[derive(Clone, Copy)]
pub(super) struct Input {
    pub resume_site: u32,
    pub available: u64,
    pub eof: bool,
}
pub(super) struct State {
    pub input: Input,
    pub paused: Option<u32>,
    borrowed_resumed: bool,
    tail_resumed: bool,
}
impl State {
    pub fn new(input: Input) -> Self {
        Self {
            input,
            paused: None,
            borrowed_resumed: false,
            tail_resumed: false,
        }
    }
}

pub(super) fn checkpoint_words(descriptor: &NativeProgramDescriptor) -> Result<usize, Error> {
    let region = descriptor
        .prefix
        .as_ref()
        .ok_or_else(|| invalid("prefix lacks same-plan proof"))?;
    let borrowed = &descriptor.borrowed_strings.regions[region.borrowed];
    (borrowed.end - borrowed.ready)
        .checked_add(3)
        .and_then(|words| words.checked_add(descriptor.max_library_inputs.max(1)))
        .ok_or_else(|| invalid("prefix checkpoint lane size overflows"))
}

pub(super) fn check_cleared(
    descriptor: &NativeProgramDescriptor,
    locals: &[u64],
    arguments: &[u64],
) -> Result<(), Error> {
    let region = descriptor
        .prefix
        .as_ref()
        .ok_or_else(|| invalid("prefix lacks same-plan proof"))?;
    let borrowed = &descriptor.borrowed_strings.regions[region.borrowed];
    if locals[borrowed.ready..borrowed.end]
        .iter()
        .any(|word| *word != 0)
        || arguments.iter().any(|word| *word != 0)
        || descriptor.closed_tails.sites[region.tail.header() as usize].is_some_and(|closed| {
            let words = descriptor.closed_tails.regions[closed].words;
            locals[words..words + 3].iter().any(|word| *word != 0)
        })
    {
        return Err(invalid("native prefix checkpoint retained borrowed words"));
    }
    Ok(())
}
impl Context<'_, '_> {
    pub(super) fn validate_prefix_extent(
        &self,
        matrix: NativeMatchValue,
        actual: usize,
    ) -> Result<(), Error> {
        let Some(prefix) = &self.prefix else {
            return Ok(());
        };
        let region = self
            .control
            .descriptor
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
            .control
            .descriptor
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
            || self.control.completed
            || self.output_cursor != 0
            || self.consumed
            || self.control.current != Some(region.frontier.iteration())
            || self
                .control
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
                .contains(&self.control.previous)
        {
            return Err(invalid("prefix outside its exact active header boundary"));
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
        self.control.work.charge(
            NativeExecutionPhase::Setup,
            amount(self.control.progress.len() / 2, 64, 1024)?,
        )?;
        self.check_prefix_header(site)?;
        if args != [0; 3] || self.window_region.is_some() || self.selected_join.is_some() {
            return Err(invalid("malformed or overlapping prefix loan"));
        }
        let region = self.control.descriptor.prefix.as_ref().unwrap();
        let borrowed = region.borrowed;
        let input = self.prefix.as_ref().unwrap().input;
        match operation {
            program_abi::PREFIX_PAUSE => {
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
                self.control.work.charge(
                    NativeExecutionPhase::Setup,
                    amount(self.argument_capacity, 16, 1024)?,
                )?;
                for word in 0..self.argument_capacity {
                    self.write_borrowed_word(word, 0)?;
                }
                self.borrowed_strings = None;
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
                let closed = self.control.descriptor.closed_tails.sites[site as usize]
                    .ok_or_else(|| invalid("prefix fuel resume outside closed Tail"))?;
                let result = self.begin_closed_region(site as usize, args, closed)?;
                self.prefix.as_mut().unwrap().tail_resumed = true;
                Ok(result)
            }
            _ => Err(invalid("unknown native prefix helper")),
        }
    }
}
