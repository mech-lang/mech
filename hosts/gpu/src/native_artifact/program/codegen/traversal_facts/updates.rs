//! Current-traversal read/update facts. No fact survives a write, unknown
//! effect, incompatible merge or latch. Different views may alias, so every
//! replacement invalidates all remembered cells, even of another array.
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CheckedRead {
    pub site: usize,
    pub output: usize,
    value: Id,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_artifact::program::codegen) struct CheckedUpdate {
    pub array: usize,
    pub index: u32,
    pub incoming: u32,
    pub read_site: usize,
}

impl State {
    pub(super) fn remember_read(
        &mut self,
        array: usize,
        index: Id,
        site: usize,
        output: usize,
        value: Id,
    ) -> Option<()> {
        let key = (array, index);
        if self.checked_reads.len() == MAX_RELATIONS && !self.checked_reads.contains_key(&key) {
            return None;
        }
        self.checked_reads.insert(
            key,
            CheckedRead {
                site,
                output,
                value,
            },
        );
        Some(())
    }

    pub(super) fn checked_increment(
        &self,
        terms: &Terms,
        array: usize,
        index: u32,
        incoming: u32,
    ) -> Option<CheckedUpdate> {
        let read = self
            .checked_reads
            .get(&(array, self.read(index as usize)?))?;
        // Retain the exact loaded scalar, not a reconstructed value of a
        // possibly overwritten source home or a cell in a previous view.
        if self.read(read.output) != Some(read.value) {
            return None;
        }
        let Expr::Arithmetic(Arithmetic::Add, a, b) = terms.terms[self.read(incoming as usize)?]
        else {
            return None;
        };
        if !self.successful.contains(&(Arithmetic::Add, a, b))
            || !((a == read.value && self.constant(terms, b) == Some(1))
                || (b == read.value && self.constant(terms, a) == Some(1)))
        {
            return None;
        }
        Some(CheckedUpdate {
            array,
            index,
            incoming,
            read_site: read.site,
        })
    }
}
