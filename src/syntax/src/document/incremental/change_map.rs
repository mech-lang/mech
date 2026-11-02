use alloc::vec::Vec;

use crate::document::{TextEdit, TextRange, TextSize};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Affinity {
    Before,
    After,
}

#[derive(Clone, Debug)]
pub struct ChangeMap {
    edits: Vec<TextEdit>,
    prefix_deltas: Vec<i64>,
    old_changed: TextRange,
    new_changed: TextRange,
}

impl ChangeMap {
    pub fn new(edits: &[TextEdit]) -> Self {
        if edits.is_empty() {
            return Self {
                edits: Vec::new(),
                prefix_deltas: alloc::vec![0],
                old_changed: TextRange::empty(TextSize::ZERO),
                new_changed: TextRange::empty(TextSize::ZERO),
            };
        }
        let old_changed = TextRange::new(edits[0].delete.start, edits[edits.len() - 1].delete.end);
        let mut prefix_deltas = alloc::vec![0_i64];
        for edit in edits {
            let delta = i64::try_from(edit.insert.len())
                .unwrap_or(i64::MAX)
                .saturating_sub(i64::from(edit.delete.len().0));
            prefix_deltas.push(prefix_deltas.last().copied().unwrap().saturating_add(delta));
        }
        let mut map = Self {
            edits: edits.to_vec(),
            prefix_deltas,
            old_changed,
            new_changed: TextRange::empty(TextSize::ZERO),
        };
        map.new_changed = TextRange::new(
            map.map_offset(old_changed.start, Affinity::Before),
            map.map_offset(old_changed.end, Affinity::After),
        );
        map
    }

    pub fn edits(&self) -> &[TextEdit] {
        &self.edits
    }

    pub fn old_changed_range(&self) -> TextRange {
        self.old_changed
    }

    pub fn new_changed_range(&self) -> TextRange {
        self.new_changed
    }

    pub fn map_range(&self, range: TextRange) -> TextRange {
        TextRange::new(
            self.map_offset(range.start, Affinity::Before),
            self.map_offset(range.end, Affinity::After),
        )
    }

    pub fn map_offset(&self, offset: TextSize, affinity: Affinity) -> TextSize {
        let completed = self.edits.partition_point(|edit| {
            edit.delete.end < offset || (edit.delete.end == offset && affinity == Affinity::After)
        });
        let delta = self.prefix_deltas[completed];
        let mapped = match self.edits.get(completed) {
            Some(edit)
                if offset > edit.delete.start
                    || (offset == edit.delete.start && affinity == Affinity::After) =>
            {
                i64::from(edit.delete.start.0)
                    .saturating_add(delta)
                    .saturating_add(if affinity == Affinity::After {
                        i64::try_from(edit.insert.len()).unwrap_or(i64::MAX)
                    } else {
                        0
                    })
            }
            _ => i64::from(offset.0).saturating_add(delta),
        };
        TextSize(mapped.clamp(0, i64::from(u32::MAX)) as u32)
    }

    /// Maps only bytes untouched by every edit. Boundary insertions belong to
    /// the adjacent changed span, never to the unchanged subtree's contents.
    pub(crate) fn map_unchanged_range(&self, range: TextRange) -> Option<TextRange> {
        let first = self
            .edits
            .partition_point(|edit| edit.delete.end <= range.start);
        if range.is_empty() {
            // Missing syntax at an edited boundary has no stable side affinity.
            if self
                .edits
                .get(first)
                .is_some_and(|edit| edit.delete.start <= range.start)
                || first
                    .checked_sub(1)
                    .and_then(|i| self.edits.get(i))
                    .is_some_and(|edit| edit.delete.end == range.start)
            {
                return None;
            }
        } else if self
            .edits
            .get(first)
            .is_some_and(|edit| edit.delete.start < range.end)
        {
            return None;
        }
        let start = self.map_offset(range.start, Affinity::After);
        let end = self.map_offset(range.end, Affinity::Before);
        (end >= start && end.0 - start.0 == range.len().0).then_some(TextRange::new(start, end))
    }

    pub fn touches_boundary(&self, range: TextRange) -> bool {
        self.edits.iter().any(|edit| {
            edit.delete.start == range.start
                || edit.delete.end == range.end
                || (edit.delete.is_empty()
                    && (edit.delete.start == range.start || edit.delete.start == range.end))
        })
    }

    pub fn changes_parser_context(
        &self,
        old_text: impl Fn(TextRange) -> alloc::string::String,
    ) -> bool {
        self.edits.iter().any(|edit| {
            let deleted = old_text(edit.delete);
            edit.insert.chars().chain(deleted.chars()).any(|character| {
                matches!(
                    character,
                    '\r' | '\n' | '`' | '~' | '(' | ')' | '[' | ']' | '{' | '}' | ':' | '='
                )
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unchanged_mapping_excludes_insertions_at_either_boundary() {
        let range = TextRange::new(TextSize(2), TextSize(4));
        let changes = ChangeMap::new(&[
            TextEdit::insert(TextSize(2), "left"),
            TextEdit::insert(TextSize(4), "right"),
        ]);
        assert_eq!(
            changes.map_unchanged_range(range),
            Some(TextRange::new(TextSize(6), TextSize(8)))
        );
        for at in [2, 4] {
            assert_eq!(
                changes.map_unchanged_range(TextRange::empty(TextSize(at))),
                None
            );
        }
        assert_eq!(
            ChangeMap::new(&[TextEdit::insert(TextSize(3), "inside")]).map_unchanged_range(range),
            None
        );
        assert_eq!(
            ChangeMap::new(&[TextEdit::delete(range)]).map_unchanged_range(range),
            None
        );
    }
}
