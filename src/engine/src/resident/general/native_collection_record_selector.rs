//! Immutable record-name facts only. Current selectors and each original
//! pre-comparison debit remain invocation-bound and are never cached here.
use super::NativeProgramLibraryTemplate;
use mech_core::{ResidentKernelError, SchemaBody};

#[derive(Debug)]
struct Name {
    id: u64,
    bytes: u64,
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
        Some(Self {
            names: names
                .into_iter()
                .map(|name| {
                    Some(Name {
                        id: mech_core::hash_str(name),
                        bytes: u64::try_from(name.len().max(1)).ok()?,
                    })
                })
                .collect::<Option<_>>()?,
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
        let prepared = RecordSelector {
            names: vec![
                Name { id: 7, bytes: 3 },
                Name { id: 7, bytes: 5 },
                Name { id: 9, bytes: 2 },
            ]
            .into_boxed_slice(),
        };
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
}
