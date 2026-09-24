//! Shared finite-product exhaustiveness accounting for source lowering and
//! artifact validation.

#[derive(Clone)]
pub(crate) enum StructuralCoverageSpace {
    Opaque,
    Bool,
    Enum(Box<[Option<StructuralCoverageSpace>]>),
    Tuple(Box<[StructuralCoverageSpace]>),
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
                .all(|pattern| matches!(pattern, StructuralCoveragePattern::Wildcard))
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
}
