use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

use super::edit::{SourceError, TextEdit, TextRange, TextSize};
use super::ids::{DocumentId, Revision};
use super::line_index::LineIndex;
use super::retained_sequence::{Measured, RetainedSequence};

#[derive(Clone, Debug)]
pub(crate) struct Piece {
    pub(crate) chunk: Arc<str>,
    pub(crate) range_in_chunk: TextRange,
}

impl Measured for Piece {
    fn measure(&self) -> usize {
        self.len().to_usize()
    }
}

impl Piece {
    pub(crate) fn len(&self) -> TextSize {
        self.range_in_chunk.len()
    }

    fn text(&self) -> &str {
        &self.chunk[self.range_in_chunk.start.to_usize()..self.range_in_chunk.end.to_usize()]
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SourceChunk<'a> {
    pub text: &'a str,
    pub range: TextRange,
}

/// Work performed inside `append_with_work`, excluding caller-owned input
/// construction and allocator headers. Node body bytes include new branch
/// descriptors; source bytes are copied only into the new shared chunk.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SourceAppendWork {
    pub accepted_bytes: u64,
    pub source_bytes_copied: u64,
    pub index_bytes_scanned: u64,
    pub piece_nodes_allocated: u64,
    pub line_nodes_allocated: u64,
    pub storage_node_body_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct TextSnapshot {
    document: DocumentId,
    revision: Revision,
    pieces: RetainedSequence<Piece>,
    byte_len: TextSize,
    line_index: LineIndex,
}

impl TextSnapshot {
    pub fn new(
        document: DocumentId,
        revision: Revision,
        source: impl Into<Arc<str>>,
    ) -> Result<Self, SourceError> {
        let source = source.into();
        let byte_len = TextSize::checked_from_usize(source.len())?;
        let line_index = LineIndex::from_str(&source);
        let pieces = if source.is_empty() {
            RetainedSequence::default()
        } else {
            [Piece {
                chunk: source,
                range_in_chunk: TextRange::new(TextSize::ZERO, byte_len),
            }]
            .into_iter()
            .collect()
        };
        Ok(Self {
            document,
            revision,
            pieces,
            byte_len,
            line_index,
        })
    }

    pub fn document(&self) -> DocumentId {
        self.document
    }

    pub fn revision(&self) -> Revision {
        self.revision
    }

    pub fn byte_len(&self) -> TextSize {
        self.byte_len
    }

    pub fn is_empty(&self) -> bool {
        self.byte_len.0 == 0
    }

    pub fn full_range(&self) -> TextRange {
        TextRange::new(TextSize::ZERO, self.byte_len)
    }

    pub fn line_index(&self) -> &LineIndex {
        &self.line_index
    }

    /// Project a retained byte boundary to one-based row and extended-grapheme
    /// column coordinates used by source consumers. Interior grapheme offsets
    /// have no such coordinate and return `None`.
    pub fn source_location(&self, offset: TextSize) -> Option<mech_core::SourceLocation> {
        self.source_locations(&[offset])?.pop()
    }

    /// Project several retained byte boundaries in one ordered pass per line.
    ///
    /// The returned locations preserve the input order. Repeated boundaries
    /// are allowed; an invalid UTF-8 or interior grapheme boundary rejects the
    /// complete projection.
    pub fn source_locations(&self, offsets: &[TextSize]) -> Option<Vec<mech_core::SourceLocation>> {
        let mut ordered = offsets.iter().copied().enumerate().collect::<Vec<_>>();
        for (_, offset) in &ordered {
            if *offset > self.byte_len() || !self.is_char_boundary(*offset) {
                return None;
            }
        }
        ordered.sort_unstable_by_key(|(_, offset)| *offset);

        let mut projected = vec![None; offsets.len()];
        let mut first = 0;
        while first < ordered.len() {
            let line = self.line_index().line_of(ordered[first].1);
            let mut last = first + 1;
            while last < ordered.len() && self.line_index().line_of(ordered[last].1) == line {
                last += 1;
            }

            let line_start = self.line_index().line_start(line)?;
            let line_end = ordered[last - 1].1;
            let mut cursor =
                super::parser::Cursor::for_range(self, TextRange::new(line_start, line_end));
            let mut col = 1_usize;
            for (output, offset) in &ordered[first..last] {
                while cursor.offset() < *offset {
                    cursor.bump_grapheme()?;
                    col = col.checked_add(1)?;
                }
                if cursor.offset() != *offset {
                    return None;
                }
                projected[*output] = Some(mech_core::SourceLocation {
                    row: line.checked_add(1)?,
                    col,
                });
            }
            first = last;
        }
        projected.into_iter().collect()
    }

    pub fn piece_count(&self) -> usize {
        self.pieces.len()
    }

    pub fn chunks(&self) -> impl Iterator<Item = &str> {
        self.pieces.iter().map(Piece::text)
    }

    pub fn text(&self, range: TextRange) -> Result<String, SourceError> {
        self.validate_range(range)?;
        let mut text = String::with_capacity(range.len().to_usize());
        self.for_each_slice(range, |slice| text.push_str(slice));
        Ok(text)
    }

    /// Compare an exact source range without allocating or copying its text.
    /// Invalid ranges and UTF-8 boundaries are rejected just as by [`Self::text`].
    pub(crate) fn text_eq(&self, range: TextRange, expected: &str) -> Result<bool, SourceError> {
        self.validate_range(range)?;
        if range.len().to_usize() != expected.len() {
            return Ok(false);
        }
        let expected = expected.as_bytes();
        let mut offset = 0;
        let mut matches = true;
        self.for_each_slice(range, |slice| {
            let end = offset + slice.len();
            matches &= slice.as_bytes() == &expected[offset..end];
            offset = end;
        });
        Ok(matches)
    }

    pub fn to_contiguous_string(&self) -> String {
        let mut text = String::with_capacity(self.byte_len.to_usize());
        for piece in self.pieces.iter() {
            text.push_str(piece.text());
        }
        text
    }

    pub fn byte_at(&self, offset: TextSize) -> Option<u8> {
        let (base, piece) = self.pieces.at_measure(offset.to_usize())?;
        piece
            .text()
            .as_bytes()
            .get(offset.to_usize() - base)
            .copied()
    }

    pub(crate) fn chunk_at(&self, offset: TextSize) -> Option<SourceChunk<'_>> {
        let (base, piece) = self.pieces.at_measure(offset.to_usize())?;
        Some(SourceChunk {
            text: piece.text(),
            range: TextRange::new(TextSize(base as u32), TextSize(base as u32) + piece.len()),
        })
    }

    pub(crate) fn chunk_before(&self, offset: TextSize) -> Option<SourceChunk<'_>> {
        if offset.0 == 0 || offset.0 > self.byte_len.0 || !self.is_char_boundary(offset) {
            return None;
        }
        let (base, piece) = self.pieces.at_measure(offset.to_usize() - 1)?;
        Some(SourceChunk {
            text: &piece.text()[..offset.to_usize() - base],
            range: TextRange::new(TextSize(base as u32), offset),
        })
    }

    pub fn is_char_boundary(&self, offset: TextSize) -> bool {
        if offset.0 == 0 || offset.0 == self.byte_len.0 {
            return true;
        }
        self.byte_at(offset)
            .map(|byte| byte & 0b1100_0000 != 0b1000_0000)
            .unwrap_or(false)
    }

    pub fn validate_range(&self, range: TextRange) -> Result<(), SourceError> {
        if range.start.0 > range.end.0 || range.end.0 > self.byte_len.0 {
            return Err(SourceError::InvalidRange(range));
        }
        if !self.is_char_boundary(range.start) {
            return Err(SourceError::InvalidUtf8Boundary(range.start));
        }
        if !self.is_char_boundary(range.end) {
            return Err(SourceError::InvalidUtf8Boundary(range.end));
        }
        Ok(())
    }

    pub fn append(&self, text: impl Into<String>) -> Result<Self, SourceError> {
        self.append_with_work(&text.into())
            .map(|(snapshot, _)| snapshot)
    }

    /// Append without copying the historical piece list or rescanning old lines.
    /// Empty input preserves revision and storage and performs no append work.
    pub fn append_with_work(&self, text: &str) -> Result<(Self, SourceAppendWork), SourceError> {
        if text.is_empty() {
            return Ok((self.clone(), SourceAppendWork::default()));
        }
        let added = TextSize::checked_from_usize(text.len())?;
        let byte_len = TextSize(
            self.byte_len
                .0
                .checked_add(added.0)
                .ok_or(SourceError::SourceTooLarge)?,
        );
        let (pieces, piece_nodes_allocated) = self.pieces.appended(Piece {
            chunk: Arc::from(text),
            range_in_chunk: TextRange::new(TextSize::ZERO, added),
        });
        let old_ends_cr = self
            .byte_len
            .0
            .checked_sub(1)
            .and_then(|offset| self.byte_at(TextSize(offset)))
            == Some(b'\r');
        let (line_index, line_nodes_allocated) =
            self.line_index.appended(self.byte_len, old_ends_cr, text);
        let work = SourceAppendWork {
            accepted_bytes: text.len() as u64,
            source_bytes_copied: text.len() as u64,
            index_bytes_scanned: text.len() as u64,
            piece_nodes_allocated,
            line_nodes_allocated,
            storage_node_body_bytes: piece_nodes_allocated
                * RetainedSequence::<Piece>::node_bytes() as u64
                + line_nodes_allocated * LineIndex::storage_node_bytes() as u64,
        };
        Ok((
            Self {
                document: self.document,
                revision: Revision(self.revision.0.saturating_add(1)),
                pieces,
                byte_len,
                line_index,
            },
            work,
        ))
    }

    pub fn apply_edits(&self, edits: &[TextEdit]) -> Result<Self, SourceError> {
        self.validate_edits(edits)?;
        if let [edit] = edits {
            if edit.delete.is_empty() && edit.delete.start == self.byte_len {
                return self
                    .append_with_work(&edit.insert)
                    .map(|(snapshot, _)| snapshot);
            }
        }
        if edits.is_empty() {
            return Ok(self.clone());
        }

        let mut pieces = Vec::with_capacity(self.pieces.len() + edits.len() * 2);
        let mut copied_until = TextSize::ZERO;
        let mut new_len = i64::from(self.byte_len.0);
        for edit in edits {
            self.copy_range(TextRange::new(copied_until, edit.delete.start), &mut pieces);
            push_insert(&mut pieces, &edit.insert)?;
            copied_until = edit.delete.end;
            new_len += i64::try_from(edit.insert.len()).map_err(|_| SourceError::SourceTooLarge)?
                - i64::from(edit.delete.len().0);
        }
        self.copy_range(TextRange::new(copied_until, self.byte_len), &mut pieces);
        let byte_len = TextSize::checked_from_usize(
            usize::try_from(new_len).map_err(|_| SourceError::SourceTooLarge)?,
        )?;
        let line_index = self.line_index.updated(self, &pieces, byte_len, edits);
        Ok(Self {
            document: self.document,
            revision: Revision(self.revision.0.saturating_add(1)),
            pieces: pieces.into_iter().collect(),
            byte_len,
            line_index,
        })
    }

    pub(crate) fn for_each_slice(&self, range: TextRange, mut f: impl FnMut(&str)) {
        if range.is_empty() {
            return;
        }
        self.pieces.visit_range(
            range.start.to_usize(),
            range.end.to_usize(),
            |base, piece| {
                let start = range.start.to_usize().saturating_sub(base);
                let end = (range.end.to_usize() - base).min(piece.len().to_usize());
                f(&piece.text()[start..end]);
            },
        );
    }

    fn validate_edits(&self, edits: &[TextEdit]) -> Result<(), SourceError> {
        let mut previous: Option<TextRange> = None;
        for edit in edits {
            self.validate_range(edit.delete)?;
            TextSize::checked_from_usize(edit.insert.len())?;
            if let Some(prior) = previous {
                if edit.delete.start.0 < prior.start.0 {
                    return Err(SourceError::UnsortedEdits);
                }
                if edit.delete.start.0 < prior.end.0 {
                    return Err(SourceError::OverlappingEdits);
                }
            }
            previous = Some(edit.delete);
        }
        Ok(())
    }

    fn copy_range(&self, range: TextRange, output: &mut Vec<Piece>) {
        if range.is_empty() {
            return;
        }
        let mut absolute = 0_u32;
        for piece in self.pieces.iter() {
            let piece_start = absolute;
            let piece_end = absolute + piece.len().0;
            absolute = piece_end;
            let start = range.start.0.max(piece_start);
            let end = range.end.0.min(piece_end);
            if start >= end {
                continue;
            }
            let local_start = piece.range_in_chunk.start.0 + start - piece_start;
            let local_end = piece.range_in_chunk.start.0 + end - piece_start;
            push_piece(
                output,
                Piece {
                    chunk: piece.chunk.clone(),
                    range_in_chunk: TextRange::new(TextSize(local_start), TextSize(local_end)),
                },
            );
        }
    }
}

#[cfg(all(test, any(feature = "std", not(feature = "no_std"))))]
mod tests {
    use super::*;
    use crate::allocation_probe;
    use crate::document::{
        GreenElement, GreenNode, GreenToken, NodeFlags, NodeId, SyntaxKind, SyntaxNode, TokenFlags,
        TokenId,
    };

    fn snapshot(text: &str, pieces: bool) -> TextSnapshot {
        if pieces {
            text.chars().fold(
                TextSnapshot::new(DocumentId(91), Revision(0), "").unwrap(),
                |snapshot, character| snapshot.append(character.to_string()).unwrap(),
            )
        } else {
            TextSnapshot::new(DocumentId(91), Revision(0), text).unwrap()
        }
    }

    #[test]
    fn append_allocations_match_path_copy_accounting_with_a_large_retained_prefix() {
        for size in [512, 4096, 16384] {
            let mut source = TextSnapshot::new(DocumentId(826), Revision(0), "").unwrap();
            for _ in 0..size {
                source = source.append("a\n").unwrap();
            }
            let ((next, work), allocations, bytes) =
                allocation_probe::measured_with_bytes(|| source.append_with_work("b\r\n").unwrap());
            let nodes = work.piece_nodes_allocated + work.line_nodes_allocated;
            assert_eq!(
                allocations as u64,
                nodes + 1,
                "only tree nodes and the new chunk allocate"
            );
            // Arc control words/alignment are deliberately separate from node
            // body accounting. This checks real allocations, not just counters.
            assert!(bytes as u64 <= work.storage_node_body_bytes + nodes * 32 + 64);
            assert!(bytes < 8192, "append copied the settled prefix: {bytes}");
            assert_eq!(next.byte_len().0, source.byte_len().0 + 3);
            assert_eq!(source.line_index().line_count(), size + 1);
        }
    }

    #[test]
    fn exact_text_comparison_preserves_range_errors_without_allocating() {
        let text = "a╭💡e\u{301}┃z\r\n";
        for pieces in [false, true] {
            let source = snapshot(text, pieces);
            assert_eq!(source.piece_count() > 1, pieces);
            // Include reversed/out-of-bounds ranges, partial code points, and empty
            // ranges. Validation must precede even an unequal expected byte length.
            for start in 0..=text.len() as u32 + 1 {
                for end in 0..=text.len() as u32 + 1 {
                    let range = TextRange::new(TextSize(start), TextSize(end));
                    for expected in [
                        "", "a", "╭", "│", "💡", "╭💡", "e\u{301}", "é", "z\r\n", text,
                    ] {
                        let oracle = source.text(range).map(|text| text == expected);
                        let (actual, allocations, bytes) =
                            allocation_probe::measured_with_bytes(|| {
                                core::hint::black_box(source.text_eq(range, expected))
                            });
                        assert_eq!(actual, oracle, "{range:?}, {expected:?}, pieces={pieces}");
                        assert_eq!((allocations, bytes), (0, 0));
                    }
                }
            }
        }
    }

    #[test]
    fn token_text_comparison_preserves_exact_piece_backed_ranges() {
        let text = "a╭💡┃z";
        for pieces in [false, true] {
            let source = snapshot(text, pieces);
            for (start, end) in [(0, 12), (1, 11), (2, 3), (0, 13), (12, 12)] {
                let range = TextRange::new(TextSize(start), TextSize(end));
                let node = SyntaxNode::new_root_at(
                    Arc::new(GreenNode {
                        id: NodeId(1),
                        kind: SyntaxKind::Expression,
                        text_len: range.len(),
                        children: crate::document::GreenChildren::from([GreenElement::Token(
                            GreenToken {
                                id: TokenId(2),
                                kind: SyntaxKind::BoxDrawing,
                                text_len: range.len(),
                                flags: if range.is_empty() {
                                    TokenFlags::MISSING
                                } else {
                                    TokenFlags::NONE
                                },
                                text_hash: 0,
                            },
                        )]),
                        flags: NodeFlags::NONE,
                        structural_hash: 0,
                    }),
                    source.clone(),
                    range.start,
                );
                let token = node.tokens().pop().unwrap();
                for expected in ["", "╭💡┃", "╭💡│", "╭", "xxxxxxxxxx", text] {
                    let oracle = token.text().map(|text| text == expected);
                    let (actual, allocations) = allocation_probe::measured(|| {
                        core::hint::black_box(token.text_eq(expected))
                    });
                    assert_eq!(actual, oracle, "{range:?}, {expected:?}, pieces={pieces}");
                    assert_eq!(allocations, 0);
                }
            }
        }
    }
}

fn push_insert(output: &mut Vec<Piece>, insert: &str) -> Result<(), SourceError> {
    if insert.is_empty() {
        return Ok(());
    }
    let len = TextSize::checked_from_usize(insert.len())?;
    push_piece(
        output,
        Piece {
            chunk: Arc::<str>::from(insert),
            range_in_chunk: TextRange::new(TextSize::ZERO, len),
        },
    );
    Ok(())
}

fn push_piece(output: &mut Vec<Piece>, piece: Piece) {
    if piece.len().0 == 0 {
        return;
    }
    if let Some(previous) = output.last_mut()
        && Arc::ptr_eq(&previous.chunk, &piece.chunk)
        && previous.range_in_chunk.end == piece.range_in_chunk.start
    {
        previous.range_in_chunk.end = piece.range_in_chunk.end;
        return;
    }
    output.push(piece);
}
