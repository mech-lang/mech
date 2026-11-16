//! Immutable record-name facts. Current selectors and funding remain bound
//! to the selected invocation; declined prefix previews use ordered debits.
use super::NativeProgramLibraryTemplate;
use crate::resident::budget::native_turn_plan_cache::NativeProjectionIndexContext;
use mech_core::{ResidentKernelError, SchemaBody};

#[derive(Debug)]
struct Name {
    id: u64,
    bytes: u64,
    first: bool,
    prefix: Option<(u64, u64)>,
}

#[derive(Debug)]
pub(super) struct RecordSelector {
    names: Box<[Name]>,
}

impl RecordSelector {
    pub(super) fn derive(template: &NativeProgramLibraryTemplate) -> Option<Self> {
        if template.primitive()
            != crate::native_artifact::NativeCollectionPrimitive::RecordFieldAccess
        {
            return None;
        }
        let SchemaBody::Record(fields) = template.inputs().first()?.body() else {
            return None;
        };
        Self::from_names(fields.iter().map(|field| field.name.as_str()))
    }

    fn from_names<'a>(names: impl IntoIterator<Item = &'a str>) -> Option<Self> {
        let names = names
            .into_iter()
            .map(|name| {
                Some((
                    mech_core::hash_str(name),
                    u64::try_from(name.len().max(1)).ok()?,
                ))
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self::from_ids(names))
    }

    fn from_ids(names: impl IntoIterator<Item = (u64, u64)>) -> Self {
        let mut prepared: Vec<Name> = Vec::new();
        let mut prefix = Some((0u64, 0u64));
        for (id, bytes) in names {
            prefix = prefix.and_then(|(compute, comparison)| {
                Some((
                    compute.checked_add(bytes.checked_mul(16)?.checked_add(64)?)?,
                    comparison.checked_add(bytes)?,
                ))
            });
            let first = !prepared.iter().any(|name| name.id == id);
            prepared.push(Name {
                id,
                bytes,
                first,
                prefix,
            });
        }
        Self {
            names: prepared.into_boxed_slice(),
        }
    }

    pub(super) fn ordinal_in_context(
        &self,
        selected: u64,
        expected: usize,
        context: &NativeProjectionIndexContext<'_>,
    ) -> Result<usize, ResidentKernelError> {
        if let Some(name) = self.names.get(expected)
            && name.id == selected
            && name.first
            && let Some((compute, comparison)) = name.prefix
            && context.try_charge_prefix(compute, comparison)
        {
            return Ok(expected);
        }
        self.ordinal(selected, |bytes| {
            context.charge(
                bytes
                    .checked_mul(16)
                    .and_then(|n| n.checked_add(64))
                    .ok_or(ResidentKernelError::InvalidShape)?,
                bytes,
            )
        })
    }

    pub(super) fn ordinal(
        &self,
        selected: u64,
        mut charge: impl FnMut(u64) -> Result<(), ResidentKernelError>,
    ) -> Result<usize, ResidentKernelError> {
        // Preserve the maintained selector's exact first-match interpretation,
        // including collisions, unknown IDs and every failed charged prefix.
        for (ordinal, name) in self.names.iter().enumerate() {
            charge(name.bytes)?;
            if name.id == selected {
                return Ok(ordinal);
            }
        }
        Err(ResidentKernelError::InvalidInput)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resident::budget::{
        NativeLibraryWorkBudget, native_turn_plan_cache::NativeProjectionIndexContext,
    };
    use mech_core::{ManagedMemoryBudget, MemoryBudgetLimits, ResourceDemand};

    #[test]
    fn prepared_names_match_maintained_selector_at_each_finite_spent_prefix() {
        let names = ["", "a", "é", "count", "longer-field-name"];
        let prepared = RecordSelector::from_names(names).unwrap();
        let mut selectors = names.map(mech_core::hash_str).to_vec();
        selectors.extend([mech_core::hash_str("absent"), mech_core::hash_str("a")]);
        let (mut successes, mut denials, mut spent_denials) = (0, 0, 0);
        for selected in selectors {
            for compute in [0, 63, 64, 79, 80, 159, 160, 287, 288, 10000] {
                for comparison in [0, 1, 2, 3, 4, 5, 8, 9, 1000] {
                    let trial = |cached| {
                        let memory = ManagedMemoryBudget::new(1 << 20);
                        let work = NativeLibraryWorkBudget::new(100000, 100000, &memory).unwrap();
                        let context = NativeProjectionIndexContext::new(
                            &work,
                            MemoryBudgetLimits {
                                max_compute_work: Some(compute),
                                max_comparison_work: Some(comparison),
                                ..Default::default()
                            },
                            ResourceDemand::default(),
                        );
                        let charge = |bytes: u64| context.charge(bytes * 16 + 64, bytes);
                        let result = if cached {
                            prepared.ordinal(selected, charge)
                        } else {
                            crate::resident::numeric::named_schema_ordinal(names, selected, charge)
                        };
                        (format!("{result:?}"), work.used())
                    };
                    let cached = trial(true);
                    assert_eq!(cached, trial(false));
                    if cached.0.starts_with("Ok(") {
                        successes += 1;
                    } else {
                        denials += 1;
                        spent_denials += usize::from(cached.1 != (0, 0));
                    }
                }
            }
        }
        assert!(successes > 0 && denials > 0 && spent_denials > 0);
    }

    #[test]
    fn duplicate_hashes_keep_first_match_and_current_selector() {
        let prepared = RecordSelector::from_ids([(7, 3), (7, 5), (9, 2)]);
        for (selected, expected, debits) in [
            (7, Some(0), vec![3]),
            (9, Some(2), vec![3, 5, 2]),
            (11, None, vec![3, 5, 2]),
            (7, Some(0), vec![3]),
        ] {
            let mut charged = Vec::new();
            let result = prepared.ordinal(selected, |bytes| {
                charged.push(bytes);
                Ok(())
            });
            assert_eq!(result.ok(), expected);
            assert_eq!(charged, debits);
        }
    }

    #[test]
    fn funded_prefix_matches_ordered_errors_and_spent_prefixes_in_nested_calls() {
        let prepared = RecordSelector::from_ids([(7, 1), (9, 3), (7, 5), (11, 2)]);
        let mut successes = 0;
        let mut paid_failures = 0;
        for selected in [7, 9, 11, 99] {
            for expected in 0..=4 {
                for compute in [0, 79, 80, 81, 191, 192, 193, 335, 336, 431, 432, 1000] {
                    for comparison in [0, 1, 2, 3, 4, 5, 8, 9, 10, 11, 100] {
                        // Each authority independently becomes the tight cap.
                        for authority in 0..4 {
                            let trial = |combined| {
                                let memory = ManagedMemoryBudget::new(1 << 20);
                                let work = NativeLibraryWorkBudget::new(
                                    if authority == 0 { compute + 17 } else { 10000 },
                                    if authority == 0 { comparison + 2 } else { 1000 },
                                    &memory,
                                )
                                .unwrap();
                                work.charge(17, 2).unwrap();
                                let result = work
                                    .with_call_limits(
                                        if authority == 1 { compute } else { 10000 },
                                        if authority == 1 { comparison } else { 1000 },
                                        || {
                                            work.with_call_limits(
                                                if authority == 2 { compute } else { 10000 },
                                                if authority == 2 { comparison } else { 1000 },
                                                || {
                                                    let context = NativeProjectionIndexContext::new(
                                                        &work,
                                                        MemoryBudgetLimits {
                                                            max_compute_work: Some(
                                                                if authority == 3 {
                                                                    compute + 13
                                                                } else {
                                                                    10000
                                                                },
                                                            ),
                                                            max_comparison_work: Some(
                                                                if authority == 3 {
                                                                    comparison + 1
                                                                } else {
                                                                    1000
                                                                },
                                                            ),
                                                            max_temporary_bytes: Some(1000),
                                                            max_retained_nodes: Some(10),
                                                            ..Default::default()
                                                        },
                                                        ResourceDemand {
                                                            work: mech_core::WorkDemand {
                                                                compute: 13,
                                                                comparison: 1,
                                                                ..Default::default()
                                                            },
                                                            turn_peak_bytes: 100,
                                                            retained_nodes: 3,
                                                            ..Default::default()
                                                        },
                                                    );
                                                    let result = if combined {
                                                        prepared.ordinal_in_context(
                                                            selected, expected, &context,
                                                        )
                                                    } else {
                                                        prepared.ordinal(selected, |bytes| {
                                                            context.charge(bytes * 16 + 64, bytes)
                                                        })
                                                    };
                                                    result.and_then(|actual| {
                                                        if actual == expected {
                                                            Ok(())
                                                        } else {
                                                            Err(ResidentKernelError::InvalidInput)
                                                        }
                                                    })
                                                },
                                            )
                                            .unwrap()
                                        },
                                    )
                                    .unwrap();
                                (result, work.used())
                            };
                            let actual = trial(true);
                            assert_eq!(
                                actual,
                                trial(false),
                                "selector={selected} ordinal={expected} compute={compute} comparison={comparison} authority={authority}"
                            );
                            successes += usize::from(actual.0.is_ok());
                            paid_failures += usize::from(actual.0.is_err() && actual.1 != (17, 2));
                        }
                    }
                }
            }
        }
        assert!(successes > 0 && paid_failures > 0);
    }

    #[test]
    fn prefix_preview_requires_same_scope_and_all_projection_demand_without_spending() {
        let memory = ManagedMemoryBudget::new(1 << 20);
        let work = NativeLibraryWorkBudget::new(1000, 100, &memory).unwrap();
        let foreign = NativeLibraryWorkBudget::new(1000, 100, &memory).unwrap();
        let limits = MemoryBudgetLimits {
            max_compute_work: Some(1000),
            max_comparison_work: Some(100),
            max_temporary_bytes: Some(10),
            max_retained_nodes: Some(3),
            ..Default::default()
        };
        let context = NativeProjectionIndexContext::new(&work, limits, ResourceDemand::default());
        assert!(!context.try_charge_prefix(80, 1));
        assert_eq!(work.used(), (0, 0));
        foreign
            .with_call_limits(1000, 100, || {
                assert!(!context.try_charge_prefix(80, 1));
                assert_eq!(
                    context.charge(80, 1),
                    Err(ResidentKernelError::InvalidInput)
                );
            })
            .unwrap();
        work.with_call_limits(1000, 100, || {
            for prior in [
                ResourceDemand {
                    turn_peak_bytes: 11,
                    ..Default::default()
                },
                ResourceDemand {
                    retained_nodes: 4,
                    ..Default::default()
                },
                ResourceDemand {
                    work: mech_core::WorkDemand {
                        compute: u64::MAX,
                        comparison: 0,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ] {
                let context = NativeProjectionIndexContext::new(&work, limits, prior);
                assert!(!context.try_charge_prefix(80, 1));
                assert_eq!(work.used(), (0, 0));
            }
            assert!(!context.try_charge_prefix(u64::MAX, 1));
            assert_eq!(work.used(), (0, 0));
            assert!(context.try_charge_prefix(80, 1));
            assert_eq!(work.used(), (80, 1));
        })
        .unwrap();
        assert_eq!(foreign.used(), (0, 0));
        // Preparation overflow is a miss, never a changed incremental error.
        let overflow = RecordSelector::from_ids([(7, 1), (9, u64::MAX)]);
        assert!(overflow.names[1].prefix.is_none());
        work.with_call_limits(1000, 100, || {
            let context =
                NativeProjectionIndexContext::new(&work, limits, ResourceDemand::default());
            assert_eq!(
                overflow.ordinal_in_context(9, 1, &context),
                Err(ResidentKernelError::InvalidShape)
            );
        })
        .unwrap();
        assert_eq!(work.used(), (160, 2));
    }

    #[test]
    fn worker_prefix_keeps_incremental_denial_and_reconciles_its_paid_prefix() {
        let trial = |combined, compute| {
            let memory = ManagedMemoryBudget::new(1 << 20);
            let root = NativeLibraryWorkBudget::new(1000, 100, &memory).unwrap();
            let group = root.reserve_worker_group(compute, 10).unwrap();
            let worker =
                NativeLibraryWorkBudget::from_worker_loan(group.worker_loan().unwrap()).unwrap();
            let prepared = RecordSelector::from_ids([(7, 1), (9, 3)]);
            let result = worker
                .with_call_limits(1000, 100, || {
                    let context = NativeProjectionIndexContext::new(
                        &worker,
                        MemoryBudgetLimits {
                            max_compute_work: Some(1000),
                            max_comparison_work: Some(100),
                            ..Default::default()
                        },
                        ResourceDemand::default(),
                    );
                    assert!(!context.try_charge_prefix(192, 4));
                    assert_eq!(worker.used(), (0, 0));
                    if combined {
                        prepared.ordinal_in_context(9, 1, &context)
                    } else {
                        prepared.ordinal(9, |bytes| context.charge(bytes * 16 + 64, bytes))
                    }
                })
                .unwrap();
            let used = worker.used();
            drop(worker);
            assert_eq!(group.finish().unwrap(), used);
            assert_eq!(root.used(), used);
            drop(root);
            assert_eq!(memory.used_bytes(), 0);
            (result, used)
        };
        for compute in [0, 79, 80, 191, 192, 193] {
            assert_eq!(trial(true, compute), trial(false, compute));
        }
        assert_eq!(trial(true, 191).1, (80, 1));
        assert_eq!(trial(true, 192), (Ok(1), (192, 4)));
    }
}
