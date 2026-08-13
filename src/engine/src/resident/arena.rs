use mech_core::InstanceEpoch;

#[derive(Clone, Debug)]
pub(crate) struct VersionedBatch<T> {
    pub(crate) buffers: [Box<[T]>; 2],
    pub(crate) epochs: [Option<InstanceEpoch>; 2],
}

impl<T: Copy> VersionedBatch<T> {
    fn activate(instances: usize, initial: T) -> Self {
        Self {
            buffers: [
                vec![initial; instances].into_boxed_slice(),
                vec![initial; instances].into_boxed_slice(),
            ],
            epochs: [Some(InstanceEpoch(0)), None],
        }
    }

    #[inline]
    fn published_index(&self, epoch: InstanceEpoch) -> usize {
        match self.epochs {
            [Some(left), _] if left == epoch => 0,
            [_, Some(right)] if right == epoch => 1,
            _ => panic!("published resident epoch has no typed buffer"),
        }
    }

    #[inline]
    fn begin_candidate(&mut self, base: InstanceEpoch, working: InstanceEpoch) -> (usize, usize) {
        let published = self.published_index(base);
        debug_assert_eq!(self.epochs[published], Some(base));
        let candidate = 1 - published;
        self.epochs[candidate] = Some(working);
        (published, candidate)
    }

    #[inline]
    fn reject(&mut self, candidate: usize, working: InstanceEpoch) {
        debug_assert_eq!(self.epochs[candidate], Some(working));
        self.epochs[candidate] = None;
    }

    fn split_buffers(&mut self, published: usize, candidate: usize) -> (&[T], &mut [T]) {
        debug_assert_ne!(published, candidate);
        if published == 0 {
            let (published_buffers, candidate_buffers) = self.buffers.split_at_mut(1);
            (&published_buffers[0], &mut candidate_buffers[0])
        } else {
            let (candidate_buffers, published_buffers) = self.buffers.split_at_mut(1);
            (&published_buffers[0], &mut candidate_buffers[0])
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct GateBArena {
    pub(crate) states: VersionedBatch<[f64; 3]>,
    pub(crate) covariances: VersionedBatch<[f64; 9]>,
}

impl GateBArena {
    pub(crate) fn activate(instances: usize) -> Self {
        Self {
            states: VersionedBatch::activate(instances, [2.0, 1.0, 0.15]),
            covariances: VersionedBatch::activate(
                instances,
                [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.05],
            ),
        }
    }

    #[inline]
    pub(crate) fn begin_candidate(
        &mut self,
        base: InstanceEpoch,
        working: InstanceEpoch,
    ) -> (usize, usize) {
        let state = self.states.begin_candidate(base, working);
        let covariance = self.covariances.begin_candidate(base, working);
        debug_assert_eq!(state, covariance);
        state
    }

    #[inline]
    pub(crate) fn reject_candidate(&mut self, candidate: usize, working: InstanceEpoch) {
        self.states.reject(candidate, working);
        self.covariances.reject(candidate, working);
    }

    pub(crate) fn split_buffers(
        &mut self,
        published: usize,
        candidate: usize,
    ) -> (&[[f64; 3]], &mut [[f64; 3]], &[[f64; 9]], &mut [[f64; 9]]) {
        let (states, candidate_states) = self.states.split_buffers(published, candidate);
        let (covariances, candidate_covariances) =
            self.covariances.split_buffers(published, candidate);
        (states, candidate_states, covariances, candidate_covariances)
    }

    pub(crate) fn candidate_state_hash(&self, candidate: usize) -> u64 {
        let mut batch_hash = 0xcbf29ce484222325_u64;
        for (state, covariance) in self.states.buffers[candidate]
            .iter()
            .zip(self.covariances.buffers[candidate].iter())
        {
            let mut state_hash = 0xcbf29ce484222325_u64;
            for value in state.iter().chain(covariance.iter()) {
                for byte in value.to_bits().to_le_bytes() {
                    state_hash ^= u64::from(byte);
                    state_hash = state_hash.wrapping_mul(0x100000001b3);
                }
            }
            batch_hash ^= state_hash;
            batch_hash = batch_hash.wrapping_mul(0x100000001b3);
        }
        batch_hash
    }

    pub(crate) fn contains_epoch(&self, epoch: InstanceEpoch) -> bool {
        self.states.epochs.contains(&Some(epoch)) || self.covariances.epochs.contains(&Some(epoch))
    }

    #[inline]
    pub(crate) fn published_state(&self, epoch: InstanceEpoch, instance: usize) -> [f64; 3] {
        self.states.buffers[self.states.published_index(epoch)][instance]
    }

    #[inline]
    pub(crate) fn published_covariance(&self, epoch: InstanceEpoch, instance: usize) -> [f64; 9] {
        self.covariances.buffers[self.covariances.published_index(epoch)][instance]
    }

    #[cfg(test)]
    pub(crate) fn candidate_state(&self, candidate: usize, instance: usize) -> [f64; 3] {
        self.states.buffers[candidate][instance]
    }

    #[cfg(test)]
    pub(crate) fn buffer_addresses(&self) -> ([usize; 2], [usize; 2]) {
        (
            [
                self.states.buffers[0].as_ptr() as usize,
                self.states.buffers[1].as_ptr() as usize,
            ],
            [
                self.covariances.buffers[0].as_ptr() as usize,
                self.covariances.buffers[1].as_ptr() as usize,
            ],
        )
    }
}
