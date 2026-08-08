use std::sync::atomic::{AtomicU64, Ordering};

use mech_runtime::__gate_b_recording::{
    GateBFixedReceipt, LedgerPermit, RecordEstimate, RetainedTurnLedger, prepare_retained,
    reserve_retained,
};
use sha2::{Digest, Sha256};

use super::contract::{EPISODE_LENGTH, trace};

pub const SIDE: usize = 64;
pub const ELEMENTS: usize = SIDE * SIDE;
pub const WRITTEN_BYTES: usize = ELEMENTS * core::mem::size_of::<f64>();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FullWriteProbe {
    pub candidate_seed_bytes: usize,
    pub candidate_written_bytes: usize,
    pub published_buffer_copy_bytes: usize,
    pub publication_store_count: usize,
    pub receipt_bytes: usize,
}

pub fn initial_values() -> Vec<f64> {
    (0..ELEMENTS)
        .map(|index| (index as f64 + 1.0) * 0.0001)
        .collect()
}

pub fn coefficients() -> Vec<f64> {
    (0..ELEMENTS)
        .map(|index| ((index % 127) as f64 + 1.0) * 0.000001)
        .collect()
}

pub fn write_next(current: &[f64], coefficient: &[f64], input: f64, next: &mut [f64]) {
    debug_assert_eq!(current.len(), ELEMENTS);
    debug_assert_eq!(coefficient.len(), ELEMENTS);
    debug_assert_eq!(next.len(), ELEMENTS);
    for index in 0..ELEMENTS {
        next[index] = current[index] * 1.000001 + coefficient[index] * input;
    }
}

pub fn buffer_hash(buffer: &[f64]) -> String {
    let mut hasher = Sha256::new();
    for value in buffer {
        hasher.update(value.to_le_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn buffer_hash64(buffer: &[f64]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for value in buffer {
        for byte in value.to_bits().to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

pub struct FullWriteEpochFixture {
    versions: [Box<[f64]>; 2],
    coefficient: Box<[f64]>,
    published_epoch: AtomicU64,
    next_epoch: u64,
    ledger: RetainedTurnLedger<GateBFixedReceipt>,
    permits: Vec<Option<LedgerPermit>>,
}

impl FullWriteEpochFixture {
    pub fn new() -> Self {
        let initial = initial_values().into_boxed_slice();
        let ledger = RetainedTurnLedger::new(
            EPISODE_LENGTH,
            EPISODE_LENGTH * GateBFixedReceipt::RETAINED_BYTES,
        )
        .expect("Gate B full-write ledger");
        let estimate = RecordEstimate {
            records: 1,
            bytes: GateBFixedReceipt::RETAINED_BYTES,
        };
        let permits = (0..EPISODE_LENGTH)
            .map(|_| Some(reserve_retained(&ledger, estimate).expect("Gate B admission")))
            .collect();
        Self {
            versions: [initial.clone(), initial],
            coefficient: coefficients().into_boxed_slice(),
            published_epoch: AtomicU64::new(0),
            next_epoch: 1,
            ledger,
            permits,
        }
    }

    pub fn run_episode(&mut self) {
        for (turn, input) in trace().iter().enumerate() {
            self.run_turn(turn, input.velocity, false)
                .expect("valid full-write turn");
        }
    }

    fn run_turn(&mut self, turn: usize, input: f64, reject: bool) -> Result<(), &'static str> {
        let base_epoch = self.published_epoch.load(Ordering::Acquire);
        let base_index = (base_epoch & 1) as usize;
        let working_epoch = self.next_epoch;
        let working_index = (working_epoch & 1) as usize;
        debug_assert_ne!(base_index, working_index);

        if base_index == 0 {
            let (published, candidate) = self.versions.split_at_mut(1);
            write_next(&published[0], &self.coefficient, input, &mut candidate[0]);
        } else {
            let (candidate, published) = self.versions.split_at_mut(1);
            write_next(&published[0], &self.coefficient, input, &mut candidate[0]);
        }
        if reject
            || !self.versions[working_index]
                .iter()
                .all(|value| value.is_finite())
        {
            return Err("forced full-write rejection");
        }

        let receipt = GateBFixedReceipt::accepted(
            base_epoch,
            working_epoch,
            buffer_hash64(&self.versions[working_index]),
            1,
            1,
            1,
        );
        let permit = self.permits[turn].take().expect("unused Gate B admission");
        let prepared = prepare_retained(&mut self.ledger, permit, receipt)
            .expect("Gate B full-write receipt preparation");
        self.published_epoch.store(working_epoch, Ordering::Release);
        prepared.append();
        self.next_epoch += 1;
        Ok(())
    }

    pub fn published(&self) -> &[f64] {
        let index = (self.published_epoch.load(Ordering::Acquire) & 1) as usize;
        &self.versions[index]
    }

    pub fn abort_output_hash(&mut self) -> String {
        let before_epoch = self.published_epoch.load(Ordering::Acquire);
        let before_hash = buffer_hash(self.published());
        assert!(self.run_turn(0, 1.0, true).is_err());
        assert_eq!(self.published_epoch.load(Ordering::Acquire), before_epoch);
        assert_eq!(buffer_hash(self.published()), before_hash);
        before_hash
    }

    pub fn probe(&self) -> FullWriteProbe {
        FullWriteProbe {
            candidate_seed_bytes: 0,
            candidate_written_bytes: WRITTEN_BYTES,
            published_buffer_copy_bytes: 0,
            publication_store_count: 1,
            receipt_bytes: GateBFixedReceipt::RETAINED_BYTES,
        }
    }
}
