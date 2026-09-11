//! Configured-budget ownership for the payloads of the existing typed lanes.
//! Fixed lane storage is already owned by the realized R5 arenas. These owners
//! retain only additional mutable payload capacity; canonical data transfers
//! its admission to the shared immutable root before publication.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use mech_core::{
    ManagedMemoryBudget, ManagedMemoryReservation, MemoryRuntimeError, MemoryRuntimeResult,
    ResidentValueKind, ResidentValueMut, ResidentValueRef,
};

use crate::memory_planner::TurnMemoryPlan;
use crate::resident::general::ResidentRegion;

fn add(left: u64, right: u64) -> MemoryRuntimeResult<u64> {
    left.checked_add(right)
        .ok_or(MemoryRuntimeError::AccountingInvariantViolation {
            dimension: "resident configured payload bytes",
            current: left,
            change: right,
        })
}

#[derive(Debug)]
pub(crate) struct ResidentPayloadOwner {
    budget: ManagedMemoryBudget,
    // These are already reserved by the original realized String envelopes.
    prepaid: u64,
    retained: Cell<u64>,
    strings: RefCell<Box<[u64]>>,
    extra: RefCell<ManagedMemoryReservation>,
    poisoned: Cell<bool>,
    _metadata: ManagedMemoryReservation,
}

impl ResidentPayloadOwner {
    pub(crate) fn new(
        budget: ManagedMemoryBudget,
        prepaid: u64,
        strings: usize,
        _snapshots: usize,
    ) -> MemoryRuntimeResult<Rc<Self>> {
        let tracked =
            (strings as u64)
                .checked_mul(8)
                .ok_or(MemoryRuntimeError::IdentityExhausted {
                    identity: "resident tracked payload metadata",
                })?;
        let metadata = budget.reserve_capacity(add(
            tracked,
            (core::mem::size_of::<Self>() + 2 * core::mem::size_of::<usize>()) as u64,
        )?)?;
        Ok(Rc::new(Self {
            extra: RefCell::new(budget.reserve_capacity(0)?),
            budget,
            prepaid,
            retained: Cell::new(0),
            strings: RefCell::new(vec![0; strings].into_boxed_slice()),
            poisoned: Cell::new(false),
            _metadata: metadata,
        }))
    }

    pub(crate) fn begin(
        self: &Rc<Self>,
        region: ResidentRegion,
    ) -> MemoryRuntimeResult<ResidentPayloadScope> {
        if self.poisoned.get() {
            return Err(MemoryRuntimeError::DomainClosed);
        }
        let metadata = self.budget.reserve_capacity(
            (core::mem::size_of::<ResidentPayloadAdmission>() + 2 * core::mem::size_of::<usize>())
                as u64,
        )?;
        Ok(ResidentPayloadScope {
            inner: Rc::new(ResidentPayloadAdmission {
                owner: self.clone(),
                region,
                reservation: RefCell::new(self.budget.reserve_capacity(0)?),
                auxiliary: Cell::new(0),
                peak: Cell::new(0),
                finished: Cell::new(false),
                last_error: RefCell::new(None),
                materializing: Cell::new(false),
                _metadata: metadata,
            }),
        })
    }

    fn available(&self) -> MemoryRuntimeResult<u64> {
        Ok(add(self.prepaid, self.extra.borrow().capacity_bytes())?
            .saturating_sub(self.retained.get()))
    }
}

#[derive(Debug)]
pub(crate) struct ResidentPayloadAdmission {
    owner: Rc<ResidentPayloadOwner>,
    region: ResidentRegion,
    reservation: RefCell<ManagedMemoryReservation>,
    auxiliary: Cell<u64>,
    peak: Cell<u64>,
    finished: Cell<bool>,
    last_error: RefCell<Option<MemoryRuntimeError>>,
    materializing: Cell<bool>,
    _metadata: ManagedMemoryReservation,
}

impl ResidentPayloadAdmission {
    pub(crate) fn admit_plan(&self, plan: &TurnMemoryPlan) -> MemoryRuntimeResult<()> {
        let payload = plan
            .call
            .as_ref()
            .and_then(|call| call.outputs.first())
            .map_or(plan.output_bytes, |output| {
                output.value.payload.required_bytes
            });
        let metadata = if self.region.kind == ResidentValueKind::Snapshot {
            (self.region.len as u64)
                .checked_mul(mech_core::Value::memory_budget_claim_metadata_bytes())
                .ok_or(MemoryRuntimeError::IdentityExhausted {
                    identity: "resident snapshot import metadata",
                })?
        } else {
            0
        };
        self.admit_peak(add(plan.demand.turn_peak_bytes.max(payload), metadata)?)?;
        self.materializing.set(true);
        Ok(())
    }

    fn admit_peak(&self, bytes: u64) -> MemoryRuntimeResult<()> {
        if self.owner.poisoned.get() {
            return Err(MemoryRuntimeError::DomainClosed);
        }
        let peak = self.peak.get().max(bytes);
        let credit = if self.region.kind == ResidentValueKind::String {
            self.owner.available()?
        } else {
            0
        };
        let required = add(peak, self.auxiliary.get())?.saturating_sub(credit);
        if let Err(error) = self.reservation.borrow_mut().resize_capacity(required) {
            *self.last_error.borrow_mut() = Some(error.clone());
            return Err(error);
        }
        self.peak.set(peak);
        Ok(())
    }
}

pub(crate) struct ResidentPayloadScope {
    inner: Rc<ResidentPayloadAdmission>,
}

impl ResidentPayloadScope {
    pub(crate) fn start(&self) {
        self.inner.materializing.set(true);
    }
    pub(crate) fn last_error(&self) -> Option<MemoryRuntimeError> {
        self.inner.last_error.borrow().clone()
    }
    pub(crate) fn admission(&self) -> Rc<ResidentPayloadAdmission> {
        self.inner.clone()
    }

    pub(crate) fn admit_auxiliary(&self, bytes: u64) -> MemoryRuntimeResult<()> {
        self.inner
            .auxiliary
            .set(add(self.inner.auxiliary.get(), bytes)?);
        self.inner.admit_peak(self.inner.peak.get())
    }

    pub(crate) fn admit_value(&self, value: &mech_core::Value) -> MemoryRuntimeResult<()> {
        if self.inner.region.kind == ResidentValueKind::Snapshot {
            let payload = value.memory_budget_admission_bytes(&self.inner.owner.budget)?;
            self.inner.admit_peak(payload)?;
        } else if self.inner.region.kind == ResidentValueKind::String {
            let bytes = match value.data() {
                mech_core::ValueData::String(value) => value.len() as u64,
                mech_core::ValueData::Matrix(matrix) => {
                    let mech_core::snapshot::SequenceView::String(values) = matrix.elements()
                    else {
                        return Err(MemoryRuntimeError::CandidateValidationFailed {
                            object: None,
                            reason: "expected String matrix input".into(),
                        });
                    };
                    values
                        .iter()
                        .try_fold(0_u64, |bytes, value| add(bytes, value.len() as u64))?
                }
                _ => {
                    return Err(MemoryRuntimeError::CandidateValidationFailed {
                        object: None,
                        reason: "expected String input".into(),
                    });
                }
            };
            self.inner.admit_peak(bytes)?;
        }
        Ok(())
    }

    pub(crate) fn admit_copy(
        &self,
        value: ResidentValueRef<'_>,
        container_bytes: u64,
    ) -> MemoryRuntimeResult<()> {
        let mut bytes = container_bytes;
        match value {
            ResidentValueRef::String(values) => {
                for value in values {
                    bytes = add(bytes, value.len() as u64)?;
                }
            }
            ResidentValueRef::Snapshot(values) => {
                for value in values.iter().flatten() {
                    bytes = add(
                        bytes,
                        value.memory_budget_admission_bytes(&self.inner.owner.budget)?,
                    )?;
                }
            }
            _ => {}
        }
        self.inner.admit_peak(bytes)?;
        Ok(())
    }

    pub(crate) fn finish(self, mut value: ResidentValueMut<'_>) -> MemoryRuntimeResult<()> {
        let result = self.finish_inner(&mut value);
        if result.is_err() {
            // All callers prepare unpublished candidate/input/scratch regions.
            // A returned allocation error abandons that candidate, not the
            // published value or the usable owner. Unwinding still follows the
            // conservative Drop path below.
            self.discard_candidate(value)?;
        }
        result
    }

    fn finish_inner(&self, value: &mut ResidentValueMut<'_>) -> MemoryRuntimeResult<()> {
        let mut total = self.inner.owner.retained.get();
        let start = self.inner.region.offset;
        match value {
            ResidentValueMut::String(values) => {
                let tracked = self.inner.owner.strings.borrow();
                for (index, value) in values.iter().enumerate() {
                    total = total.checked_sub(tracked[start + index]).ok_or(
                        MemoryRuntimeError::IdentityExhausted {
                            identity: "resident tracked string bytes",
                        },
                    )?;
                    total = add(total, value.capacity() as u64)?;
                }
            }
            ResidentValueMut::Snapshot(values) => {
                for value in values.iter_mut() {
                    if let Some(owned) = value.take() {
                        *value = Some(
                            owned.into_memory_budget(&mut self.inner.reservation.borrow_mut())?,
                        );
                    }
                }
            }
            _ => {}
        }
        let required = total.saturating_sub(self.inner.owner.prepaid);
        let mut extra = self.inner.owner.extra.borrow_mut();
        if required > extra.capacity_bytes() {
            let growth = self
                .inner
                .reservation
                .borrow_mut()
                .split_capacity(required - extra.capacity_bytes())?;
            extra.merge_capacity(growth)?;
        } else {
            extra.resize_capacity(required)?;
        }
        // No counter changes precede complete ownership transfer/admission.
        match value {
            ResidentValueMut::String(values) => {
                let mut tracked = self.inner.owner.strings.borrow_mut();
                for (index, value) in values.iter().enumerate() {
                    tracked[start + index] = value.capacity() as u64;
                }
            }
            _ => {}
        }
        self.inner.owner.retained.set(total);
        self.inner.finished.set(true);
        Ok(())
    }

    fn discard_candidate(&self, value: ResidentValueMut<'_>) -> MemoryRuntimeResult<()> {
        let start = self.inner.region.offset;
        let end = start + self.inner.region.len;
        let tracked = match value {
            ResidentValueMut::String(values) => {
                for value in values {
                    *value = String::new();
                }
                &self.inner.owner.strings
            }
            ResidentValueMut::Snapshot(values) => {
                for value in values {
                    *value = None;
                }
                self.inner.finished.set(true);
                return Ok(());
            }
            _ => {
                self.inner.finished.set(true);
                return Ok(());
            }
        };
        let mut tracked = tracked.borrow_mut();
        let mut total = self.inner.owner.retained.get();
        for previous in &mut tracked[start..end] {
            total = total.checked_sub(*previous).ok_or(
                MemoryRuntimeError::AccountingInvariantViolation {
                    dimension: "resident discarded candidate payload",
                    current: total,
                    change: *previous,
                },
            )?;
            *previous = 0;
        }
        self.inner
            .owner
            .extra
            .borrow_mut()
            .resize_capacity(total.saturating_sub(self.inner.owner.prepaid))?;
        self.inner.owner.retained.set(total);
        self.inner.finished.set(true);
        Ok(())
    }
}

impl Drop for ResidentPayloadScope {
    fn drop(&mut self) {
        if self.inner.finished.get() || !self.inner.materializing.get() {
            return;
        }
        // On unwind, retain the entire candidate bound with its owning arena.
        // A later call cannot treat stale per-element measurements as credit.
        self.inner.owner.poisoned.set(true);
        let mut pending = self.inner.reservation.borrow_mut();
        let bytes = pending.capacity_bytes();
        if bytes != 0 {
            let charge = pending
                .split_capacity(bytes)
                .expect("pending capacity is owned");
            self.inner
                .owner
                .extra
                .borrow_mut()
                .merge_capacity(charge)
                .expect("one payload owner shares one budget");
        }
    }
}

pub(crate) fn is_payload(kind: ResidentValueKind) -> bool {
    matches!(
        kind,
        ResidentValueKind::String | ResidentValueKind::Snapshot
    )
}

/// Concrete copy cost of the existing owned activation-input representation.
pub(crate) fn clone_bytes(value: ResidentValueRef<'_>) -> MemoryRuntimeResult<u64> {
    let (count, slot) = match value {
        ResidentValueRef::Bool(values) => (values.len(), core::mem::size_of::<u8>()),
        ResidentValueRef::Index(values) => (values.len(), core::mem::size_of::<u64>()),
        ResidentValueRef::F64(values) => (values.len(), core::mem::size_of::<f64>()),
        ResidentValueRef::String(values) => (values.len(), core::mem::size_of::<String>()),
        ResidentValueRef::Snapshot(values) => (
            values.len(),
            core::mem::size_of::<Option<mech_core::Value>>(),
        ),
    };
    let mut bytes =
        (count as u64)
            .checked_mul(slot as u64)
            .ok_or(MemoryRuntimeError::IdentityExhausted {
                identity: "resident owned input bytes",
            })?;
    match value {
        ResidentValueRef::String(values) => {
            for value in values {
                bytes = add(bytes, value.len() as u64)?;
            }
        }
        _ => {}
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::snapshot::{SnapshotValidationContext, ValueDataDraft, ValueDraft};
    use mech_core::{ResidentShape, SchemaBody, SchemaDraft, SchemaTableBuilder, Value};

    fn region(kind: ResidentValueKind, len: usize) -> ResidentRegion {
        ResidentRegion {
            kind,
            offset: 0,
            len,
            shape: ResidentShape {
                rows: 1,
                columns: len as u32,
            },
        }
    }

    fn snapshot(text: &str) -> Value {
        let mut schemas = SchemaTableBuilder::new();
        let handle = schemas
            .insert(
                SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: SchemaBody::String,
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let schemas = schemas.finish().unwrap();
        let schema = schemas.resolve(handle).unwrap();
        let (schemas, _) = schemas.into_parts();
        ValueDraft {
            schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::String(text.to_owned()),
        }
        .finalize(&SnapshotValidationContext::new(&schemas))
        .unwrap()
    }

    #[test]
    fn string_owner_reuses_prepaid_capacity_and_releases_growth_after_shrink() {
        let budget = ManagedMemoryBudget::new(1_000_000);
        let prepaid = budget.reserve_capacity(8).unwrap();
        let owner = ResidentPayloadOwner::new(budget.clone(), 8, 1, 0).unwrap();
        let baseline = budget.used_bytes();
        let mut target = [String::new()];
        for (text, extra) in [("four", 0), ("twelve bytes", 4), ("ok", 0)] {
            let source = [text.to_owned()];
            let scope = owner.begin(region(ResidentValueKind::String, 1)).unwrap();
            scope
                .admit_copy(ResidentValueRef::String(&source), 0)
                .unwrap();
            scope.start();
            target[0] = source[0].clone();
            scope.finish(ResidentValueMut::String(&mut target)).unwrap();
            assert_eq!(budget.used_bytes(), baseline + extra);
            assert_eq!(target[0], text);
        }
        drop(target);
        drop(owner);
        drop(prepaid);
        assert_eq!(budget.used_bytes(), 0);
    }

    #[test]
    fn snapshot_import_allocation_failure_discards_candidate_and_allows_retry() {
        for failure_after in [0, 1, 2, 3] {
            let budget = ManagedMemoryBudget::new(1_000_000);
            let owner = ResidentPayloadOwner::new(budget.clone(), 0, 0, 2).unwrap();
            let baseline = budget.used_bytes();
            let original = [
                Some(snapshot("first caller root")),
                Some(snapshot("second caller root")),
            ];
            let mut candidate = [None, None];
            let scope = owner.begin(region(ResidentValueKind::Snapshot, 2)).unwrap();
            scope
                .admit_copy(ResidentValueRef::Snapshot(&original), 0)
                .unwrap();
            scope.start();
            candidate.clone_from_slice(&original);
            budget.inject_snapshot_import_failure_after(failure_after);
            assert!(matches!(
                scope.finish(ResidentValueMut::Snapshot(&mut candidate)),
                Err(MemoryRuntimeError::AllocationFailed { .. })
            ));
            assert!(candidate.iter().all(Option::is_none));
            assert_eq!(budget.used_bytes(), baseline);
            assert!(!owner.poisoned.get());
            for value in original.iter().flatten() {
                assert_eq!(value.memory_budget_retained_bytes(&budget), None);
            }

            let retry = owner.begin(region(ResidentValueKind::Snapshot, 2)).unwrap();
            retry
                .admit_copy(ResidentValueRef::Snapshot(&original), 0)
                .unwrap();
            retry.start();
            candidate.clone_from_slice(&original);
            retry
                .finish(ResidentValueMut::Snapshot(&mut candidate))
                .unwrap();
            let exported = candidate[0].clone().unwrap();
            drop(candidate);
            drop(owner);
            assert!(budget.used_bytes() > 0);
            assert!(
                matches!(exported.data(), mech_core::ValueData::String(value) if value.as_ref() == "first caller root")
            );
            drop(exported);
            assert_eq!(budget.used_bytes(), 0);
        }
    }
}
