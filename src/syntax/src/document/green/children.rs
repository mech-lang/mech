//! Persistent logical children. Storage branches are never grammar nodes.
use super::*;
use crate::document::retained_sequence::{Measured, RetainedSequence};
use alloc::vec::Vec;
impl Measured for GreenElement {
    fn measure(&self) -> usize {
        self.text_len().to_usize()
    }
}
#[derive(Clone, Debug, Default)]
pub struct GreenChildren {
    items: RetainedSequence<GreenElement>,
    text_len: TextSize,
    hash: u64,
    hash_kind: Option<SyntaxKind>,
    flags: NodeFlags,
}
impl GreenChildren {
    pub(crate) fn for_kind(kind: SyntaxKind) -> Self {
        Self {
            hash: hash_u64(FNV_OFFSET, kind as u64),
            hash_kind: Some(kind),
            ..Self::default()
        }
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn get(&self, index: usize) -> Option<&GreenElement> {
        self.items.get(index)
    }
    pub fn first(&self) -> Option<&GreenElement> {
        self.get(0)
    }
    pub fn last(&self) -> Option<&GreenElement> {
        self.len().checked_sub(1).and_then(|index| self.get(index))
    }
    pub fn iter(&self) -> impl Iterator<Item = &GreenElement> {
        self.items.iter()
    }
    pub(crate) fn into_values(self) -> impl Iterator<Item = GreenElement> {
        self.items.into_values()
    }
    pub fn to_vec(&self) -> Vec<GreenElement> {
        self.iter().cloned().collect()
    }
    pub(crate) fn append(&mut self, child: GreenElement) -> u64 {
        self.text_len += child.text_len();
        if self.hash_kind.is_some() {
            self.hash = child_hash_append(self.hash, &child);
        }
        self.flags |= propagated_flags(
            SyntaxKind::Document,
            NodeFlags::NONE,
            core::slice::from_ref(&child),
        );
        let (items, allocated) = self.items.appended(child);
        self.items = items;
        allocated
    }
    pub(crate) fn text_len(&self) -> TextSize {
        self.text_len
    }
    pub(crate) fn hash(&self, kind: SyntaxKind) -> u64 {
        assert_eq!(
            self.hash_kind,
            Some(kind),
            "materialization starts with its canonical node kind"
        );
        self.hash
    }
    pub(crate) fn flags(&self, kind: SyntaxKind, explicit: NodeFlags) -> NodeFlags {
        propagated_flags(kind, explicit, &[]) | self.flags
    }
}
impl core::ops::Index<usize> for GreenChildren {
    type Output = GreenElement;
    fn index(&self, index: usize) -> &GreenElement {
        &self.items[index]
    }
}
impl FromIterator<GreenElement> for GreenChildren {
    fn from_iter<I: IntoIterator<Item = GreenElement>>(iter: I) -> Self {
        let mut children = Self::default();
        for child in iter {
            children.append(child);
        }
        children
    }
}
impl From<Vec<GreenElement>> for GreenChildren {
    fn from(value: Vec<GreenElement>) -> Self {
        value.into_iter().collect()
    }
}
impl<const N: usize> From<[GreenElement; N]> for GreenChildren {
    fn from(value: [GreenElement; N]) -> Self {
        value.into_iter().collect()
    }
}
