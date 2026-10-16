//! Private, admitted byte boundaries shared by ordinary String selectors.
use core::ops::Range;

#[derive(Debug)]
pub(crate) struct PreparedGraphemeIndex {
    boundaries: Vec<usize>,
}

impl PreparedGraphemeIndex {
    /// The allocator receives the byte-only upper bound before segmentation.
    /// It must return an empty vector with at least that admitted capacity.
    pub(crate) fn build_with<E>(
        text: &str,
        allocate: impl FnOnce(usize) -> Result<Vec<usize>, E>,
        invalid: impl Fn() -> E,
    ) -> Result<Self, E> {
        let capacity = text.len().checked_add(1).ok_or_else(&invalid)?;
        let mut boundaries = allocate(capacity)?;
        if !boundaries.is_empty() || boundaries.capacity() < capacity {
            return Err(invalid());
        }
        boundaries.push(0);
        let mut position = 0usize;
        for part in grapheme::Graphemes::from_usvs(text).iter() {
            position = position
                .checked_add(part.as_str().len())
                .ok_or_else(&invalid)?;
            boundaries.push(position);
        }
        if position != text.len() {
            return Err(invalid());
        }
        Ok(Self { boundaries })
    }

    pub(crate) fn len(&self) -> usize {
        self.boundaries.len() - 1
    }

    pub(crate) fn byte_range(&self, ordinal: usize) -> Option<Range<usize>> {
        Some(*self.boundaries.get(ordinal)?..*self.boundaries.get(ordinal.checked_add(1)?)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boundary_allocation_is_requested_before_segmentation_and_preserves_raw_bytes() {
        for (source, expected) in [
            ("", vec![]),
            ("éA", vec!["é", "A"]),
            (
                "e\u{301}🧑‍🚀\r\n🇺🇸\0",
                vec!["e\u{301}", "🧑‍🚀", "\r\n", "🇺🇸", "\0"],
            ),
        ] {
            let index = PreparedGraphemeIndex::build_with(
                source,
                |capacity| {
                    assert_eq!(capacity, source.len() + 1);
                    Ok::<_, ()>(Vec::with_capacity(capacity))
                },
                || (),
            )
            .unwrap();
            assert_eq!(index.len(), expected.len());
            for (ordinal, expected) in expected.iter().enumerate() {
                assert_eq!(&source[index.byte_range(ordinal).unwrap()], *expected);
            }
            assert!(index.byte_range(expected.len()).is_none());
        }
        let denied = PreparedGraphemeIndex::build_with(
            "é",
            |_| Err::<Vec<usize>, _>("denied"),
            || "invalid",
        );
        assert_eq!(denied.unwrap_err(), "denied");
    }
}
