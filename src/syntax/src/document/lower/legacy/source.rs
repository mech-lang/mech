use mech_core::{SourceLocation, SourceRange};

use crate::document::parser::Cursor;
use crate::document::{TextRange, TextSize, TextSnapshot};

pub(super) fn source_location(source: &TextSnapshot, offset: TextSize) -> Option<SourceLocation> {
    if offset > source.byte_len() || !source.is_char_boundary(offset) {
        return None;
    }

    let line = source.line_index().line_of(offset);
    let line_start = source.line_index().line_start(line)?;
    let mut cursor = Cursor::for_range(source, TextRange::new(line_start, offset));
    let mut column = 1_usize;
    while cursor.bump_grapheme().is_some() {
        column = column.checked_add(1)?;
    }
    if cursor.offset() != offset {
        return None;
    }

    Some(SourceLocation {
        row: line.checked_add(1)?,
        col: column,
    })
}

pub(super) fn source_range(source: &TextSnapshot, range: TextRange) -> Option<SourceRange> {
    source.validate_range(range).ok()?;
    Some(SourceRange {
        start: source_location(source, range.start)?,
        end: source_location(source, range.end)?,
    })
}
