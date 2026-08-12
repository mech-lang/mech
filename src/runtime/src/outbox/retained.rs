use std::collections::VecDeque;

use mech_core::{MResult, MechError};

use crate::{
    ledger::{
        CapacityController, CapacityReservation, LedgerAllocationFailed, RecordEstimate,
        invalid_permit, prepare_reservation, reserve,
    },
    turn_record::AccountedRecord,
};

use super::{
    DuplicateOutboxEffectId, InvalidOutboxEffectOrder, OutboxEffectId, OutboxPermit,
    OwnedEffectIntent,
};

#[derive(Debug)]
pub struct PreparedOutboxBatch<P> {
    controller: CapacityController,
    reservation: Option<CapacityReservation>,
    effects: Option<Vec<(usize, OwnedEffectIntent<P>)>>,
}

impl<P> PreparedOutboxBatch<P> {
    pub fn len(&self) -> usize {
        self.effects.as_ref().map_or(0, Vec::len)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn into_parts(mut self) -> (CapacityReservation, Vec<(usize, OwnedEffectIntent<P>)>) {
        let reservation = self
            .reservation
            .take()
            .expect("prepared outbox reservation consumed once");
        let effects = self
            .effects
            .take()
            .expect("prepared outbox effects consumed once");
        (reservation, effects)
    }
}

impl<P> Drop for PreparedOutboxBatch<P> {
    fn drop(&mut self) {
        if let Some(reservation) = self.reservation.take() {
            self.controller.release_prepared(reservation);
        }
    }
}

/// A bounded FIFO outbox retaining effect intents in deterministic ID order.
#[derive(Debug)]
pub struct RetainedEffectOutbox<P> {
    effects: VecDeque<OwnedEffectIntent<P>>,
    effect_bytes: VecDeque<usize>,
    controller: CapacityController,
    last_appended_id: Option<OutboxEffectId>,
}

impl<P> RetainedEffectOutbox<P> {
    pub fn new(max_effects: usize, max_bytes: usize) -> MResult<Self> {
        let controller = CapacityController::new(max_effects, max_bytes)?;
        let mut effects = VecDeque::new();
        effects.try_reserve_exact(max_effects).map_err(|_| {
            MechError::new(
                LedgerAllocationFailed {
                    resource: "outbox effect slots",
                },
                None,
            )
        })?;
        let mut effect_bytes = VecDeque::new();
        effect_bytes.try_reserve_exact(max_effects).map_err(|_| {
            MechError::new(
                LedgerAllocationFailed {
                    resource: "outbox accounting slots",
                },
                None,
            )
        })?;
        Ok(Self {
            effects,
            effect_bytes,
            controller,
            last_appended_id: None,
        })
    }

    pub fn len(&self) -> usize {
        self.effects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    pub fn retained_bytes(&self) -> usize {
        self.controller.retained().bytes
    }

    pub fn iter(&self) -> impl Iterator<Item = &OwnedEffectIntent<P>> {
        self.effects.iter()
    }

    pub fn front(&self) -> Option<&OwnedEffectIntent<P>> {
        self.effects.front()
    }

    pub fn front_mut(&mut self) -> Option<&mut OwnedEffectIntent<P>> {
        self.effects.front_mut()
    }

    pub(crate) fn reserve(&self, estimate: RecordEstimate) -> MResult<OutboxPermit> {
        Ok(OutboxPermit {
            inner: Some(reserve(&self.controller, estimate)?),
        })
    }

    pub(crate) fn prepare_batch(
        &self,
        mut permit: OutboxPermit,
        mut effects: Vec<OwnedEffectIntent<P>>,
    ) -> MResult<PreparedOutboxBatch<P>>
    where
        P: AccountedRecord + Send + 'static,
    {
        effects.sort_unstable_by_key(|effect| effect.id);
        if let Some(duplicate) = effects
            .windows(2)
            .find(|pair| pair[0].id == pair[1].id)
            .map(|pair| pair[0].id)
        {
            return Err(MechError::new(
                DuplicateOutboxEffectId { id: duplicate },
                None,
            ));
        }
        let expected_previous = self.last_appended_id;
        if let (Some(previous), Some(next)) =
            (expected_previous, effects.first().map(|effect| effect.id))
        {
            if next <= previous {
                return Err(MechError::new(
                    InvalidOutboxEffectOrder { previous, next },
                    None,
                ));
            }
        }
        let mut accounted = Vec::new();
        accounted.try_reserve_exact(effects.len()).map_err(|_| {
            MechError::new(
                LedgerAllocationFailed {
                    resource: "prepared outbox batch",
                },
                None,
            )
        })?;
        let mut total_bytes = 0_usize;
        for effect in effects {
            let bytes = effect.validated_retained_bytes()?;
            total_bytes = total_bytes.checked_add(bytes).ok_or_else(|| {
                MechError::new(
                    LedgerAllocationFailed {
                        resource: "outbox byte accounting",
                    },
                    None,
                )
            })?;
            accounted.push((bytes, effect));
        }
        let permit = permit
            .inner
            .take()
            .ok_or_else(|| invalid_permit("outbox permit has already been consumed"))?;
        let (controller, reservation) =
            prepare_reservation(&self.controller, permit, accounted.len(), total_bytes)?;
        Ok(PreparedOutboxBatch {
            controller,
            reservation: Some(reservation),
            effects: Some(accounted),
        })
    }

    /// Transfers a prepared batch without allocation or a recoverable failure branch.
    pub(crate) fn append(&mut self, prepared: PreparedOutboxBatch<P>) {
        let reservation = *prepared
            .reservation
            .as_ref()
            .expect("live prepared outbox reservation");
        self.controller
            .commit_prepared(&prepared.controller, reservation);
        let (_, effects) = prepared.into_parts();
        let last_appended_id = effects.last().map(|(_, effect)| effect.id);
        for (bytes, effect) in effects {
            self.effects.push_back(effect);
            self.effect_bytes.push_back(bytes);
        }
        if let Some(last_appended_id) = last_appended_id {
            self.last_appended_id = Some(last_appended_id);
        }
    }

    pub fn pop_front(&mut self) -> Option<OwnedEffectIntent<P>> {
        let effect = self.effects.pop_front()?;
        let bytes = self
            .effect_bytes
            .pop_front()
            .expect("retained outbox byte accounting entry");
        self.controller.release_retained(1, bytes);
        Some(effect)
    }

    pub fn acknowledge_front(&mut self) -> Option<OwnedEffectIntent<P>> {
        self.pop_front()
    }

    pub fn drain(&mut self) -> impl Iterator<Item = OwnedEffectIntent<P>> + '_ {
        std::iter::from_fn(|| self.pop_front())
    }
}
