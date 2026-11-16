//! Iterate over available registers.

use crate::{MachineEnv, PReg, PRegSet, PRegSetIter, RegClass};

/// Keep track of where we are in the register traversal.
struct Cursor {
    first: PRegSetIter,
    second: PRegSetIter,
}

impl Cursor {
    #[inline]
    fn new(registers: &PRegSet, class: RegClass, offset_hint: usize) -> Self {
        let mut mask = PRegSet::empty();
        mask.add_up_to(PReg::new(offset_hint % PReg::MAX, class));
        let first = *registers & mask.invert();
        let second = *registers & mask;
        Self {
            first: first.into_iter(),
            second: second.into_iter(),
        }
    }

    fn next(&mut self) -> Option<PReg> {
        self.first.next().or_else(|| self.second.next())
    }
}

/// This iterator represents a traversal through all allocatable registers of a
/// given class, in a certain order designed to minimize allocation contention.
///
/// The order in which we try registers is somewhat complex:
/// - First, if the register is fixed (i.e., pre-assigned), return that and stop
///   iteration.
/// - Then, if there is a hint, try that one.
/// - Next we try registers in a traversal order that is based on an "offset"
///   (usually the bundle index) spreading pressure evenly among registers to
///   reduce commitment-map contention.
/// - Within that scan, we try registers in two groups: first, preferred
///   registers; then, non-preferred registers. (In normal usage, these consist
///   of caller-save and callee-save registers respectively, to minimize
///   clobber-saves; but they need not.)
pub struct RegTraversalIter {
    is_fixed: bool,
    fixed: Option<PReg>,
    use_hint: bool,
    hint: Option<PReg>,
    preferred: Cursor,
    non_preferred: Cursor,
    limit: Option<usize>,
}

impl RegTraversalIter {
    pub fn new(
        env: &MachineEnv,
        class: RegClass,
        fixed: Option<PReg>,
        hint: Option<PReg>,
        offset: usize,
        limit: Option<usize>,
    ) -> Self {
        debug_assert!(fixed != Some(PReg::invalid()));
        debug_assert!(hint != Some(PReg::invalid()));

        let class_index = class as u8 as usize;
        let preferred = Cursor::new(&env.preferred_regs_by_class[class_index], class, offset);
        let non_preferred =
            Cursor::new(&env.non_preferred_regs_by_class[class_index], class, offset);

        Self {
            is_fixed: fixed.is_some(),
            fixed,
            use_hint: hint.is_some(),
            hint,
            preferred,
            non_preferred,
            limit,
        }
    }
}

impl core::iter::Iterator for RegTraversalIter {
    type Item = PReg;

    fn next(&mut self) -> Option<PReg> {
        if self.is_fixed {
            return self.fixed.take();
        }

        if self.use_hint {
            self.use_hint = false;
            if self.hint.unwrap().hw_enc() < self.limit.unwrap_or(usize::MAX) {
                return self.hint;
            }
        }

        while let Some(reg) = self.preferred.next() {
            if Some(reg) == self.hint || reg.hw_enc() >= self.limit.unwrap_or(usize::MAX) {
                continue; // Try again; we already tried the hint or we are outside of the register range limit.
            }
            return Some(reg);
        }

        while let Some(reg) = self.non_preferred.next() {
            if Some(reg) == self.hint || reg.hw_enc() >= self.limit.unwrap_or(usize::MAX) {
                continue; // Try again; we already tried the hint or we are outside of the register range limit.
            }
            return Some(reg);
        }

        None
    }
}
