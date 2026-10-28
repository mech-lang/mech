//! Compile-time current-visit facts, one bit per definition. No runtime state.
use std::ops::{Index, Range};

#[derive(Clone)]
pub(crate) struct DefinitionSet {
    len: usize,
    words: Vec<u64>,
}

impl DefinitionSet {
    fn words(len: usize) -> Option<usize> {
        len.checked_add(63).map(|n| n / 64)
    }

    pub(super) fn storage_bytes(rows: usize, cells: usize) -> Option<usize> {
        Self::words(cells)?
            .checked_mul(std::mem::size_of::<u64>())?
            .checked_add(std::mem::size_of::<Self>())?
            .checked_mul(rows)
    }

    pub(super) fn new(len: usize) -> Option<Self> {
        Some(Self {
            len,
            words: vec![0; Self::words(len)?],
        })
    }

    pub(super) fn empty() -> Self {
        Self {
            len: 0,
            words: Vec::new(),
        }
    }

    pub(super) fn get(&self, index: usize) -> Option<&bool> {
        (index < self.len).then(|| {
            if self.words[index / 64] & (1u64 << (index % 64)) != 0 {
                &true
            } else {
                &false
            }
        })
    }

    pub(super) fn set(&mut self, index: usize, value: bool) -> Result<(), &'static str> {
        if index >= self.len {
            return Err("definition outside current-visit set");
        }
        let mask = 1u64 << (index % 64);
        if value {
            self.words[index / 64] |= mask;
        } else {
            self.words[index / 64] &= !mask;
        }
        Ok(())
    }

    pub(super) fn intersect(&mut self, other: &Self) -> Result<(), &'static str> {
        if self.len != other.len {
            return Err("current-visit set lengths differ");
        }
        for (word, other) in self.words.iter_mut().zip(&other.words) {
            *word &= other;
        }
        Ok(())
    }

    pub(super) fn last_set(&self, range: Range<usize>) -> Option<usize> {
        assert!(range.start <= range.end && range.end <= self.len);
        let start = range.start;
        range
            .rev()
            .find(|&index| self[index])
            .map(|index| index - start)
    }

    pub(super) fn same_suffix(&self, other: &Self, start: usize) -> bool {
        if self.len != other.len || start > self.len {
            return false;
        }
        if start == self.len {
            return true;
        }
        let word = start / 64;
        let mask = u64::MAX << (start % 64);
        self.words[word] & mask == other.words[word] & mask
            && self.words[word + 1..] == other.words[word + 1..]
    }

    pub(super) fn any_suffix(&self, start: usize) -> bool {
        assert!(start <= self.len);
        if start == self.len {
            return false;
        }
        let word = start / 64;
        self.words[word] & (u64::MAX << (start % 64)) != 0
            || self.words[word + 1..].iter().any(|word| *word != 0)
    }
}

impl Index<usize> for DefinitionSet {
    type Output = bool;
    fn index(&self, index: usize) -> &bool {
        self.get(index).expect("validated current-visit index")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_and_lexical_suffixes_agree_across_machine_word_boundaries() {
        for len in [0, 1, 63, 64, 65, 127, 128, 129, 4097] {
            let left = (0..len).map(|i| i % 3 != 0).collect::<Vec<_>>();
            let right = (0..len).map(|i| i % 5 != 0).collect::<Vec<_>>();
            let mut a = DefinitionSet::new(len).unwrap();
            let mut b = DefinitionSet::new(len).unwrap();
            for i in 0..len {
                a.set(i, left[i]).unwrap();
                b.set(i, right[i]).unwrap();
            }
            let mut joined = a.clone();
            joined.intersect(&b).unwrap();
            for i in 0..len {
                assert_eq!(joined[i], left[i] && right[i]);
            }
            for start in 0..=len {
                assert_eq!(a.same_suffix(&b, start), left[start..] == right[start..]);
                assert_eq!(a.any_suffix(start), left[start..].iter().any(|v| *v));
                assert_eq!(
                    a.last_set(start..len),
                    left[start..].iter().rposition(|v| *v)
                );
            }
            for i in 0..len {
                joined.set(i, false).unwrap();
            }
            assert!(!joined.any_suffix(0));
            assert!(joined.set(len, true).is_err());
        }
    }

    #[test]
    fn proof_storage_remains_finite_and_accounts_for_each_row_owner() {
        let bytes = DefinitionSet::storage_bytes(6430, 2900).unwrap();
        assert!(6430usize * 2900 > 16 * 1024 * 1024);
        assert!(bytes < 16 * 1024 * 1024);
        assert_eq!(
            DefinitionSet::storage_bytes(1, 64),
            Some(8 + std::mem::size_of::<DefinitionSet>())
        );
        assert_eq!(
            DefinitionSet::storage_bytes(1, 65),
            Some(16 + std::mem::size_of::<DefinitionSet>())
        );
        assert_eq!(DefinitionSet::storage_bytes(usize::MAX, 65), None);
        assert_eq!(DefinitionSet::storage_bytes(1, usize::MAX), None);
        assert!(DefinitionSet::storage_bytes(1_000_000, 1).unwrap() > 16 * 1024 * 1024);
    }
}
