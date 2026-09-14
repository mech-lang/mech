use mech_core::{SourceLocation, SourceRange};

use crate::document::{TextRange, TextSize, TextSnapshot};

pub(super) fn source_location(source: &TextSnapshot, offset: TextSize) -> Option<SourceLocation> {
    source.source_location(offset)
}

pub(super) fn source_range(source: &TextSnapshot, range: TextRange) -> Option<SourceRange> {
    source.validate_range(range).ok()?;
    Some(SourceRange {
        start: source_location(source, range.start)?,
        end: source_location(source, range.end)?,
    })
}
