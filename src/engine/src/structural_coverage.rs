//! Shared finite-product exhaustiveness accounting for source lowering and
//! artifact validation.

#[derive(Clone)]
pub(crate) enum StructuralCoverageSpace {
    Opaque,
    Bool,
    Enum(Box<[Option<StructuralCoverageSpace>]>),
    Tuple(Box<[StructuralCoverageSpace]>),
    Repeated {
        element: Box<StructuralCoverageSpace>,
        count: usize,
    },
}

#[derive(Clone)]
pub(crate) enum StructuralCoveragePattern {
    Never,
    Wildcard,
    Bool(bool),
    Enum {
        ordinal: u32,
        payload: Option<Box<StructuralCoveragePattern>>,
    },
    Tuple(Box<[StructuralCoveragePattern]>),
    Sequence {
        prefix: Box<[StructuralCoveragePattern]>,
        wildcard_count: usize,
        suffix: Box<[StructuralCoveragePattern]>,
    },
}

pub(crate) struct StructuralPatternCoverage {
    space: StructuralCoverageSpace,
    patterns: Vec<StructuralCoveragePattern>,
}

impl StructuralPatternCoverage {
    pub(crate) fn new(space: StructuralCoverageSpace) -> Self {
        Self {
            space,
            patterns: Vec::new(),
        }
    }

    pub(crate) fn cover_all(&mut self) {
        self.patterns.push(StructuralCoveragePattern::Wildcard);
    }

    pub(crate) fn cover_bool(&mut self, value: bool) {
        self.patterns.push(StructuralCoveragePattern::Bool(value));
    }

    pub(crate) fn cover(&mut self, pattern: StructuralCoveragePattern) {
        self.patterns.push(pattern);
    }

    pub(crate) fn is_complete(&self) -> bool {
        let rows = self
            .patterns
            .iter()
            .cloned()
            .map(|pattern| vec![pattern])
            .collect::<Vec<_>>();
        rows_are_complete(core::slice::from_ref(&self.space), &rows)
    }
}

fn rows_are_complete(
    spaces: &[StructuralCoverageSpace],
    rows: &[Vec<StructuralCoveragePattern>],
) -> bool {
    if rows.is_empty() {
        return false;
    }
    if rows.iter().any(|row| {
        row.len() == spaces.len()
            && row
                .iter()
                .zip(spaces)
                .all(|(pattern, space)| pattern.covers_all(space))
    }) {
        return true;
    }
    let Some((space, remaining_spaces)) = spaces.split_first() else {
        return true;
    };
    match space {
        StructuralCoverageSpace::Opaque => {
            let specialized = rows
                .iter()
                .filter_map(|row| match row.split_first() {
                    Some((StructuralCoveragePattern::Wildcard, rest)) => Some(rest.to_vec()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            rows_are_complete(remaining_spaces, &specialized)
        }
        StructuralCoverageSpace::Bool => [false, true].into_iter().all(|value| {
            let specialized = rows
                .iter()
                .filter_map(|row| {
                    let (pattern, rest) = row.split_first()?;
                    match pattern {
                        StructuralCoveragePattern::Wildcard => Some(rest.to_vec()),
                        StructuralCoveragePattern::Bool(candidate) if *candidate == value => {
                            Some(rest.to_vec())
                        }
                        _ => None,
                    }
                })
                .collect::<Vec<_>>();
            rows_are_complete(remaining_spaces, &specialized)
        }),
        StructuralCoverageSpace::Enum(variants) => {
            variants.iter().enumerate().all(|(ordinal, payload_space)| {
                let mut specialized_spaces = Vec::with_capacity(
                    remaining_spaces.len() + usize::from(payload_space.is_some()),
                );
                if let Some(payload_space) = payload_space {
                    specialized_spaces.push(payload_space.clone());
                }
                specialized_spaces.extend_from_slice(remaining_spaces);
                let specialized = rows
                    .iter()
                    .filter_map(|row| {
                        let (pattern, rest) = row.split_first()?;
                        let mut specialized = Vec::with_capacity(specialized_spaces.len());
                        match pattern {
                            StructuralCoveragePattern::Wildcard => {
                                if payload_space.is_some() {
                                    specialized.push(StructuralCoveragePattern::Wildcard);
                                }
                            }
                            StructuralCoveragePattern::Enum {
                                ordinal: candidate,
                                payload,
                            } if usize::try_from(*candidate).ok() == Some(ordinal) => {
                                match (payload_space, payload) {
                                    (None, None) => {}
                                    (Some(_), Some(payload)) => {
                                        specialized.push((**payload).clone())
                                    }
                                    _ => return None,
                                }
                            }
                            _ => return None,
                        }
                        specialized.extend_from_slice(rest);
                        Some(specialized)
                    })
                    .collect::<Vec<_>>();
                rows_are_complete(&specialized_spaces, &specialized)
            })
        }
        StructuralCoverageSpace::Tuple(fields) => {
            let mut specialized_spaces = fields.to_vec();
            specialized_spaces.extend_from_slice(remaining_spaces);
            let specialized = rows
                .iter()
                .filter_map(|row| {
                    let (pattern, rest) = row.split_first()?;
                    let mut specialized = Vec::with_capacity(specialized_spaces.len());
                    match pattern {
                        StructuralCoveragePattern::Wildcard => specialized.extend(
                            core::iter::repeat(StructuralCoveragePattern::Wildcard)
                                .take(fields.len()),
                        ),
                        StructuralCoveragePattern::Tuple(items) if items.len() == fields.len() => {
                            specialized.extend_from_slice(items)
                        }
                        _ => return None,
                    }
                    specialized.extend_from_slice(rest);
                    Some(specialized)
                })
                .collect::<Vec<_>>();
            rows_are_complete(&specialized_spaces, &specialized)
        }
        StructuralCoverageSpace::Repeated { element, count } => {
            if *count == 0 {
                let specialized = rows
                    .iter()
                    .filter_map(|row| {
                        let (pattern, rest) = row.split_first()?;
                        match pattern {
                            StructuralCoveragePattern::Wildcard => Some(rest.to_vec()),
                            StructuralCoveragePattern::Sequence {
                                prefix,
                                wildcard_count: 0,
                                suffix,
                            } if prefix.is_empty() && suffix.is_empty() => Some(rest.to_vec()),
                            _ => None,
                        }
                    })
                    .collect::<Vec<_>>();
                return rows_are_complete(remaining_spaces, &specialized);
            }
            let active = rows
                .iter()
                .filter_map(|row| {
                    let (pattern, _) = row.split_first()?;
                    pattern
                        .leading_repeated_wildcards(element, *count)
                        .map(|leading| (row, leading))
                })
                .collect::<Vec<_>>();
            let Some(skip) = active.iter().map(|(_, leading)| *leading).min() else {
                return false;
            };
            if skip > 0 {
                let remaining_count = count - skip;
                let mut specialized_spaces = Vec::with_capacity(remaining_spaces.len() + 1);
                if remaining_count > 0 {
                    specialized_spaces.push(StructuralCoverageSpace::Repeated {
                        element: element.clone(),
                        count: remaining_count,
                    });
                }
                specialized_spaces.extend_from_slice(remaining_spaces);
                let specialized = active
                    .into_iter()
                    .filter_map(|(row, _)| {
                        let (pattern, rest) = row.split_first()?;
                        let mut specialized = Vec::with_capacity(specialized_spaces.len());
                        if let Some(pattern) =
                            pattern.strip_repeated_wildcards(element, *count, skip)?
                        {
                            specialized.push(pattern);
                        }
                        specialized.extend_from_slice(rest);
                        Some(specialized)
                    })
                    .collect::<Vec<_>>();
                return rows_are_complete(&specialized_spaces, &specialized);
            }
            let mut specialized_spaces = Vec::with_capacity(remaining_spaces.len() + 2);
            specialized_spaces.push((**element).clone());
            specialized_spaces.push(StructuralCoverageSpace::Repeated {
                element: element.clone(),
                count: count - 1,
            });
            specialized_spaces.extend_from_slice(remaining_spaces);
            let specialized = rows
                .iter()
                .filter_map(|row| {
                    let (pattern, rest) = row.split_first()?;
                    let mut specialized = Vec::with_capacity(specialized_spaces.len());
                    match pattern {
                        StructuralCoveragePattern::Wildcard => {
                            specialized.push(StructuralCoveragePattern::Wildcard);
                            specialized.push(StructuralCoveragePattern::Wildcard);
                        }
                        StructuralCoveragePattern::Sequence {
                            prefix,
                            wildcard_count,
                            suffix,
                        } if prefix
                            .len()
                            .checked_add(*wildcard_count)
                            .and_then(|length| length.checked_add(suffix.len()))
                            == Some(*count) =>
                        {
                            let (head, next_prefix, next_wildcard_count, next_suffix) =
                                if let Some((head, rest)) = prefix.split_first() {
                                    (
                                        head.clone(),
                                        rest.to_vec().into_boxed_slice(),
                                        *wildcard_count,
                                        suffix.clone(),
                                    )
                                } else if *wildcard_count > 0 {
                                    (
                                        StructuralCoveragePattern::Wildcard,
                                        Vec::new().into_boxed_slice(),
                                        *wildcard_count - 1,
                                        suffix.clone(),
                                    )
                                } else {
                                    let (head, rest) = suffix.split_first()?;
                                    (
                                        head.clone(),
                                        Vec::new().into_boxed_slice(),
                                        0,
                                        rest.to_vec().into_boxed_slice(),
                                    )
                                };
                            specialized.push(head);
                            specialized.push(StructuralCoveragePattern::Sequence {
                                prefix: next_prefix,
                                wildcard_count: next_wildcard_count,
                                suffix: next_suffix,
                            });
                        }
                        _ => return None,
                    }
                    specialized.extend_from_slice(rest);
                    Some(specialized)
                })
                .collect::<Vec<_>>();
            rows_are_complete(&specialized_spaces, &specialized)
        }
    }
}

impl StructuralCoveragePattern {
    fn covers_all(&self, space: &StructuralCoverageSpace) -> bool {
        match (self, space) {
            (Self::Wildcard, _) => true,
            (
                Self::Sequence {
                    prefix,
                    wildcard_count,
                    suffix,
                },
                StructuralCoverageSpace::Repeated { element, count },
            ) => {
                prefix
                    .len()
                    .checked_add(*wildcard_count)
                    .and_then(|length| length.checked_add(suffix.len()))
                    == Some(*count)
                    && prefix
                        .iter()
                        .chain(suffix.iter())
                        .all(|pattern| pattern.covers_all(element))
            }
            _ => false,
        }
    }

    fn leading_repeated_wildcards(
        &self,
        element: &StructuralCoverageSpace,
        count: usize,
    ) -> Option<usize> {
        match self {
            Self::Never => None,
            Self::Wildcard => Some(count),
            Self::Sequence {
                prefix,
                wildcard_count,
                suffix,
            } if prefix
                .len()
                .checked_add(*wildcard_count)
                .and_then(|length| length.checked_add(suffix.len()))
                == Some(count) =>
            {
                let prefix_wildcards = prefix
                    .iter()
                    .take_while(|pattern| pattern.covers_all(element))
                    .count();
                if prefix_wildcards != prefix.len() {
                    return Some(prefix_wildcards);
                }
                prefix_wildcards.checked_add(*wildcard_count)?.checked_add(
                    suffix
                        .iter()
                        .take_while(|pattern| pattern.covers_all(element))
                        .count(),
                )
            }
            _ => Some(0),
        }
    }

    fn strip_repeated_wildcards(
        &self,
        element: &StructuralCoverageSpace,
        count: usize,
        mut skip: usize,
    ) -> Option<Option<Self>> {
        match self {
            Self::Wildcard => Some((skip < count).then_some(Self::Wildcard)),
            Self::Sequence {
                prefix,
                wildcard_count,
                suffix,
            } => {
                let total = prefix
                    .len()
                    .checked_add(*wildcard_count)?
                    .checked_add(suffix.len())?;
                if total != count {
                    return None;
                }
                let prefix_skip = prefix
                    .iter()
                    .take_while(|pattern| pattern.covers_all(element))
                    .count()
                    .min(skip);
                skip -= prefix_skip;
                let wildcard_skip = (*wildcard_count).min(skip);
                skip -= wildcard_skip;
                let suffix_skip = suffix
                    .iter()
                    .take_while(|pattern| pattern.covers_all(element))
                    .count()
                    .min(skip);
                skip -= suffix_skip;
                if skip != 0 {
                    return None;
                }
                let remaining = count
                    .checked_sub(prefix_skip)?
                    .checked_sub(wildcard_skip)?
                    .checked_sub(suffix_skip)?;
                if remaining == 0 {
                    return Some(None);
                }
                Some(Some(Self::Sequence {
                    prefix: prefix[prefix_skip..].to_vec().into_boxed_slice(),
                    wildcard_count: *wildcard_count - wildcard_skip,
                    suffix: suffix[suffix_skip..].to_vec().into_boxed_slice(),
                }))
            }
            Self::Never => None,
            _ => (skip == 0).then(|| Some(self.clone())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuple_coverage_tracks_the_finite_product_without_losing_correlations() {
        let tuple = StructuralCoverageSpace::Tuple(
            vec![StructuralCoverageSpace::Bool, StructuralCoverageSpace::Bool].into_boxed_slice(),
        );
        let pattern =
            |left, right| StructuralCoveragePattern::Tuple(vec![left, right].into_boxed_slice());
        let mut coverage = StructuralPatternCoverage::new(tuple);
        coverage.patterns = vec![
            pattern(
                StructuralCoveragePattern::Bool(true),
                StructuralCoveragePattern::Bool(true),
            ),
            pattern(
                StructuralCoveragePattern::Bool(false),
                StructuralCoveragePattern::Bool(false),
            ),
        ];
        assert!(!coverage.is_complete());

        coverage.patterns = vec![
            pattern(
                StructuralCoveragePattern::Bool(true),
                StructuralCoveragePattern::Wildcard,
            ),
            pattern(
                StructuralCoveragePattern::Bool(false),
                StructuralCoveragePattern::Wildcard,
            ),
        ];
        assert!(coverage.is_complete());
    }

    #[test]
    fn repeated_coverage_tracks_each_finite_element() {
        let mut coverage = StructuralPatternCoverage::new(StructuralCoverageSpace::Repeated {
            element: Box::new(StructuralCoverageSpace::Bool),
            count: 1,
        });
        coverage.patterns = vec![
            StructuralCoveragePattern::Sequence {
                prefix: vec![StructuralCoveragePattern::Bool(true)].into_boxed_slice(),
                wildcard_count: 0,
                suffix: Box::new([]),
            },
            StructuralCoveragePattern::Sequence {
                prefix: vec![StructuralCoveragePattern::Bool(false)].into_boxed_slice(),
                wildcard_count: 0,
                suffix: Box::new([]),
            },
        ];
        assert!(coverage.is_complete());
    }

    #[test]
    fn repeated_coverage_skips_large_shared_wildcard_runs_symbolically() {
        let mut coverage = StructuralPatternCoverage::new(StructuralCoverageSpace::Repeated {
            element: Box::new(StructuralCoverageSpace::Bool),
            count: 1_000_000,
        });
        coverage.patterns = [true, false]
            .into_iter()
            .map(|value| StructuralCoveragePattern::Sequence {
                prefix: Box::new([]),
                wildcard_count: 999_999,
                suffix: vec![StructuralCoveragePattern::Bool(value)].into_boxed_slice(),
            })
            .collect();
        assert!(coverage.is_complete());
    }
}
