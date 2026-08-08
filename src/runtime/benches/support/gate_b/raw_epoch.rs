use std::sync::atomic::{AtomicU64, Ordering};

use mech_runtime::__gate_b_recording::{
    GateBFixedReceipt, InputSequence, InputSequenceRange, LedgerPermit, OwnedTurnRecord,
    RecordEstimate, RetainedTurnLedger, TurnId, TurnRecordHeader, TurnRecordStatus,
    prepare_retained, reserve_retained,
};
use mech_runtime::TransactionId;

use super::contract::{
    EPISODE_LENGTH, EkfState, assert_state_close, quantized_trajectory_hash, reference_trajectory,
    state_hash64, trace,
};
use super::raw_kernel::{self, IntegrityError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EpochProbe {
    pub candidate_seed_bytes: usize,
    pub candidate_written_bytes: usize,
    pub published_buffer_copy_bytes: usize,
    pub publication_store_count: usize,
    pub receipt_bytes: usize,
    pub record_preparation_count: usize,
    pub record_append_count: usize,
    pub records_appended: usize,
    pub ledger_records_inspected: usize,
    pub post_publication_append_infallible: bool,
}

pub struct EpochFixture {
    versions: [Vec<EkfState>; 2],
    published_epoch: AtomicU64,
    next_epoch: u64,
    ledger: RetainedTurnLedger<OwnedTurnRecord<GateBFixedReceipt>>,
    permits: Vec<Option<LedgerPermit>>,
    probe: EpochProbe,
}

impl EpochFixture {
    pub fn new(instances: usize) -> Self {
        let ledger = RetainedTurnLedger::new(
            EPISODE_LENGTH,
            EPISODE_LENGTH * GateBFixedReceipt::RETAINED_BYTES,
        )
        .expect("Gate B retained ledger");
        let estimate = RecordEstimate {
            records: 1,
            bytes: GateBFixedReceipt::RETAINED_BYTES,
        };
        let permits = (0..EPISODE_LENGTH)
            .map(|_| Some(reserve_retained(&ledger, estimate).expect("Gate B admission")))
            .collect();
        Self {
            versions: [
                vec![EkfState::INITIAL; instances],
                vec![EkfState::INITIAL; instances],
            ],
            published_epoch: AtomicU64::new(0),
            next_epoch: 1,
            ledger,
            permits,
            probe: EpochProbe {
                candidate_seed_bytes: 0,
                candidate_written_bytes: instances * core::mem::size_of::<EkfState>(),
                published_buffer_copy_bytes: 0,
                publication_store_count: 1,
                receipt_bytes: GateBFixedReceipt::RETAINED_BYTES,
                record_preparation_count: 1,
                record_append_count: 1,
                records_appended: EPISODE_LENGTH,
                ledger_records_inspected: 0,
                post_publication_append_infallible: true,
            },
        }
    }

    pub fn run_episode(&mut self) {
        for (turn, input) in trace().iter().copied().enumerate() {
            self.run_turn(turn, input).expect("frozen EKF epoch turn");
        }
    }

    pub fn run_and_validate_every_turn(&mut self) -> String {
        let mut trajectory = Vec::with_capacity(EPISODE_LENGTH);
        for (turn, (input, expected)) in trace()
            .iter()
            .copied()
            .zip(reference_trajectory().iter().copied())
            .enumerate()
        {
            self.run_turn(turn, input).expect("frozen EKF epoch turn");
            for actual in self.published_states() {
                assert_state_close(*actual, expected, turn + 1);
            }
            trajectory.push(self.published_states()[0]);
        }
        quantized_trajectory_hash(&trajectory)
    }

    fn run_turn(
        &mut self,
        turn: usize,
        input: super::contract::EkfInput,
    ) -> Result<(), IntegrityError> {
        let base_epoch = self.published_epoch.load(Ordering::Acquire);
        let base_index = (base_epoch & 1) as usize;
        let working_epoch = self.next_epoch;
        let working_index = (working_epoch & 1) as usize;
        debug_assert_ne!(base_index, working_index);

        for index in 0..self.versions[base_index].len() {
            self.versions[working_index][index] =
                raw_kernel::step(self.versions[base_index][index], input)?;
        }
        let mut state_hash = 0xcbf29ce484222325_u64;
        for state in &self.versions[working_index] {
            state_hash ^= state_hash64(*state);
            state_hash = state_hash.wrapping_mul(0x100000001b3);
        }
        let identity = u64::try_from(turn + 1).expect("Gate B turn identity");
        let input_sequence = InputSequence::new(identity).expect("non-zero Gate B input");
        let receipt = OwnedTurnRecord {
            header: TurnRecordHeader {
                turn_id: TurnId::new(identity).expect("non-zero Gate B turn"),
                transaction_id: TransactionId::new(u128::from(identity)),
                input_range: Some(
                    InputSequenceRange::new(input_sequence, input_sequence)
                        .expect("one-input Gate B range"),
                ),
                status: TurnRecordStatus::Accepted,
                failure: None,
            },
            body: GateBFixedReceipt::accepted(
                base_epoch,
                working_epoch,
                state_hash,
                u16::try_from(self.versions[working_index].len() * 2)
                    .expect("Gate B touched count"),
                u16::try_from(self.versions[working_index].len() * 2)
                    .expect("Gate B changed count"),
                u16::try_from(self.versions[working_index].len() * 15).expect("Gate B dirty count"),
            ),
        };
        let permit = self.permits[turn].take().expect("unused Gate B admission");
        let prepared = prepare_retained(&mut self.ledger, permit, receipt)
            .expect("Gate B exact receipt preparation");
        self.published_epoch.store(working_epoch, Ordering::Release);
        prepared.append();
        self.next_epoch += 1;
        Ok(())
    }

    pub fn published_states(&self) -> &[EkfState] {
        let index = (self.published_epoch.load(Ordering::Acquire) & 1) as usize;
        &self.versions[index]
    }

    pub fn published_epoch(&self) -> u64 {
        self.published_epoch.load(Ordering::Acquire)
    }

    pub fn retained_receipts(&self) -> usize {
        self.ledger.len()
    }

    pub fn probe(&self) -> EpochProbe {
        self.probe
    }

    pub fn force_rejected_turn_preserves_publication(&mut self) {
        let before_epoch = self.published_epoch();
        let before = self.published_states().to_vec();
        let mut input = trace()[0];
        input.measured_range = f64::NAN;
        assert!(self.run_turn(0, input).is_err());
        assert_eq!(self.published_epoch(), before_epoch);
        assert_eq!(self.published_states(), before);
    }
}
