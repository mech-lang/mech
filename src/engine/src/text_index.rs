//! Private, admitted byte boundaries shared by ordinary String selectors.
use core::ops::Range;

#[derive(Debug)]
pub(crate) enum PreparedGraphemeIndex {
    Offsets(Vec<usize>),
    // Proven by canonical segmentation, NOT by an ASCII assumption (CRLF).
    #[cfg(any(
        test,
        all(
            feature = "native-artifact",
            any(
                feature = "pure_text",
                feature = "map_ops",
                feature = "sequence_buffers"
            )
        )
    ))]
    UnitBoundaries {
        count: usize,
    },
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
        let boundaries = allocate(capacity)?;
        if !boundaries.is_empty() || boundaries.capacity() < capacity {
            return Err(invalid());
        }
        Self::from_boundaries(text, boundaries, invalid)
    }

    /// Each scan is admitted before its first grapheme/byte inspection.
    /// The first pass proves whether byte ordinals ARE grapheme ordinals.
    /// Non-unit text uses the same canonical segmenter a second time after
    /// the caller has admitted the exact counted offset capacity.
    #[cfg(any(
        test,
        all(
            feature = "native-artifact",
            any(
                feature = "pure_text",
                feature = "map_ops",
                feature = "sequence_buffers"
            )
        )
    ))]
    pub(crate) fn build_compact_with<E>(
        text: &str,
        mut admit_scan: impl FnMut(usize) -> Result<(), E>,
        allocate: impl FnOnce(usize) -> Result<Vec<usize>, E>,
        invalid: impl Fn() -> E,
    ) -> Result<Self, E> {
        let scan = text.len().checked_add(1).ok_or_else(&invalid)?;
        admit_scan(scan)?;
        let mut count = 0usize;
        let mut unit = true;
        for part in grapheme::Graphemes::from_usvs(text).iter() {
            count = count.checked_add(1).ok_or_else(&invalid)?;
            unit &= part.as_str().len() == 1;
        }
        if unit {
            if count != text.len() {
                return Err(invalid());
            }
            return Ok(Self::UnitBoundaries { count });
        }
        admit_scan(scan)?;
        let capacity = count.checked_add(1).ok_or_else(&invalid)?;
        let boundaries = allocate(capacity)?;
        if !boundaries.is_empty() || boundaries.capacity() < capacity {
            return Err(invalid());
        }
        Self::from_boundaries(text, boundaries, invalid)
    }

    fn from_boundaries<E>(
        text: &str,
        mut boundaries: Vec<usize>,
        invalid: impl Fn() -> E,
    ) -> Result<Self, E> {
        boundaries.push(0);
        let mut position = 0usize;
        for part in grapheme::Graphemes::from_usvs(text).iter() {
            position = position
                .checked_add(part.as_str().len())
                .ok_or_else(&invalid)?;
            // The caller admitted complete capacity before segmentation.
            if boundaries.len() == boundaries.capacity() {
                return Err(invalid());
            }
            boundaries.push(position);
        }
        if position != text.len() {
            return Err(invalid());
        }
        Ok(Self::Offsets(boundaries))
    }

    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Offsets(boundaries) => boundaries.len() - 1,
            #[cfg(any(
                test,
                all(
                    feature = "native-artifact",
                    any(
                        feature = "pure_text",
                        feature = "map_ops",
                        feature = "sequence_buffers"
                    )
                )
            ))]
            Self::UnitBoundaries { count } => *count,
        }
    }

    pub(crate) fn byte_range(&self, ordinal: usize) -> Option<Range<usize>> {
        match self {
            Self::Offsets(boundaries) => {
                Some(*boundaries.get(ordinal)?..*boundaries.get(ordinal.checked_add(1)?)?)
            }
            #[cfg(any(
                test,
                all(
                    feature = "native-artifact",
                    any(
                        feature = "pure_text",
                        feature = "map_ops",
                        feature = "sequence_buffers"
                    )
                )
            ))]
            Self::UnitBoundaries { count } if ordinal < *count => {
                Some(ordinal..ordinal.checked_add(1)?)
            }
            #[cfg(any(
                test,
                all(
                    feature = "native-artifact",
                    any(
                        feature = "pure_text",
                        feature = "map_ops",
                        feature = "sequence_buffers"
                    )
                )
            ))]
            Self::UnitBoundaries { .. } => None,
        }
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

#[cfg(test)]
mod compact_tests {
    use super::*;
    use core::cell::Cell;

    #[test]
    fn compact_boundaries_use_canonical_graphemes_and_admit_every_scan_first() {
        for (text, expected, unit) in [
            ("", vec![], true),
            ("A\0Z", vec!["A", "\0", "Z"], true),
            ("a\rb", vec!["a", "\r", "b"], true),
            ("\r\n", vec!["\r\n"], false),
            ("éA", vec!["é", "A"], false),
            ("e\u{301}🧑‍🚀🇺🇸\0", vec!["e\u{301}", "🧑‍🚀", "🇺🇸", "\0"], false),
        ] {
            let scans = Cell::new(0);
            let allocations = Cell::new(0);
            let index = PreparedGraphemeIndex::build_compact_with(
                text,
                |work| {
                    assert_eq!(work, text.len() + 1);
                    scans.set(scans.get() + 1);
                    Ok::<_, ()>(())
                },
                |capacity| {
                    assert_eq!(
                        scans.get(),
                        2,
                        "both scans must be prepaid before offset allocation"
                    );
                    assert_eq!(capacity, expected.len() + 1);
                    allocations.set(allocations.get() + 1);
                    Ok(Vec::with_capacity(capacity))
                },
                || (),
            )
            .unwrap();
            assert_eq!(index.len(), expected.len());
            assert_eq!(scans.get(), if unit { 1 } else { 2 });
            assert_eq!(allocations.get(), usize::from(!unit));
            assert_eq!(
                matches!(index, PreparedGraphemeIndex::UnitBoundaries { .. }),
                unit
            );
            for (ordinal, expected) in expected.into_iter().enumerate() {
                assert_eq!(&text[index.byte_range(ordinal).unwrap()], expected);
            }
            assert!(index.byte_range(index.len()).is_none());
        }
        let scans = Cell::new(0);
        let allocation = Cell::new(false);
        let result = PreparedGraphemeIndex::build_compact_with(
            "é",
            |_| {
                scans.set(scans.get() + 1);
                Err("scan denied")
            },
            |capacity| {
                allocation.set(true);
                Ok(Vec::with_capacity(capacity))
            },
            || "invalid",
        );
        assert_eq!(result.unwrap_err(), "scan denied");
        assert_eq!(scans.get(), 1);
        assert!(!allocation.get());
        let scans = Cell::new(0);
        let result = PreparedGraphemeIndex::build_compact_with(
            "é",
            |_| {
                scans.set(scans.get() + 1);
                if scans.get() == 2 {
                    Err("second scan denied")
                } else {
                    Ok(())
                }
            },
            |capacity| {
                allocation.set(true);
                Ok(Vec::with_capacity(capacity))
            },
            || "invalid",
        );
        assert_eq!(result.unwrap_err(), "second scan denied");
        assert!(!allocation.get());
    }
}
