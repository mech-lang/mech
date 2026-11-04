//! Must-current facts with shared ordered identities and packed truth words.
//! Unavailable identities remain in the domain: a missing return is not proof
//! that every returned identity is current. Only the truth representation is
//! compacted; meet and strengthening retain the left state's exact key domain.
use super::Key;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone, Default)]
pub(super) struct State {
    keys: Arc<Vec<Key>>,
    words: Vec<u64>,
}

impl State {
    pub fn keys(&self) -> std::slice::Iter<'_, Key> {
        self.keys.iter()
    }

    pub fn get(&self, key: &Key) -> Option<bool> {
        self.keys
            .binary_search(key)
            .ok()
            .map(|index| self.bit(index))
    }

    fn bit(&self, index: usize) -> bool {
        self.words[index / 64] & (1u64 << (index % 64)) != 0
    }

    fn set(&mut self, index: usize, value: bool) {
        let mask = 1u64 << (index % 64);
        if value {
            self.words[index / 64] |= mask;
        } else {
            self.words[index / 64] &= !mask;
        }
    }

    pub fn insert(&mut self, key: Key, value: bool) {
        let index = match self.keys.binary_search(&key) {
            Ok(index) => index,
            Err(index) => {
                let old_len = self.keys.len();
                self.words.resize((old_len + 1).div_ceil(64), 0);
                for position in (index..old_len).rev() {
                    self.set(position + 1, self.bit(position));
                }
                // Tree states already contain the complete identity universe.
                // Leaf paths may introduce an identity; clone its shared domain
                // before growing so siblings keep their original bit positions.
                Arc::make_mut(&mut self.keys).insert(index, key);
                index
            }
        };
        self.set(index, value);
    }

    pub fn clear_where(&mut self, mut predicate: impl FnMut(&Key) -> bool) {
        for index in 0..self.keys.len() {
            if predicate(&self.keys[index]) {
                self.set(index, false);
            }
        }
    }

    /// Meet every incoming path, reporting exactly whether a left fact changed.
    pub fn meet(&mut self, other: &Self) -> bool {
        self.combine(other, false)
    }

    /// Combine independently established theorems over the existing domain.
    pub fn strengthen(&mut self, other: &Self) {
        self.combine(other, true);
    }

    fn combine(&mut self, other: &Self, strengthen: bool) -> bool {
        let mut changed = false;
        if Arc::ptr_eq(&self.keys, &other.keys) || self.keys == other.keys {
            for (current, &proven) in self.words.iter_mut().zip(&other.words) {
                let value = if strengthen {
                    *current | proven
                } else {
                    *current & proven
                };
                changed |= value != *current;
                *current = value;
            }
        } else {
            // Sparse leaf paths can have different domains. Missing source
            // facts still mean unavailable, and no right-only key is imported.
            for index in 0..self.keys.len() {
                let current = self.bit(index);
                let proven = other.get(&self.keys[index]).unwrap_or(false);
                let value = if strengthen {
                    current | proven
                } else {
                    current & proven
                };
                changed |= value != current;
                self.set(index, value);
            }
        }
        changed
    }
}

impl FromIterator<(Key, bool)> for State {
    fn from_iter<T: IntoIterator<Item = (Key, bool)>>(iter: T) -> Self {
        // Preserve ordered-map duplicate semantics (the final definition wins).
        // This is one universe construction, not a map copied per CFG state.
        let ordered = iter.into_iter().collect::<BTreeMap<_, _>>();
        let mut words = vec![0; ordered.len().div_ceil(64)];
        let keys = ordered
            .into_iter()
            .enumerate()
            .map(|(index, (key, value))| {
                if value {
                    words[index / 64] |= 1u64 << (index % 64);
                }
                key
            })
            .collect();
        Self {
            keys: Arc::new(keys),
            words,
        }
    }
}

impl Extend<(Key, bool)> for State {
    fn extend<T: IntoIterator<Item = (Key, bool)>>(&mut self, iter: T) {
        for (key, value) in iter {
            self.insert(key, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exact(compact: &State, ordinary: &BTreeMap<Key, bool>) {
        assert_eq!(
            compact.keys().copied().collect::<Vec<_>>(),
            ordinary.keys().copied().collect::<Vec<_>>()
        );
        for key in ordinary.keys() {
            assert_eq!(compact.get(key), ordinary.get(key).copied(), "{key:?}");
        }
        assert_eq!(compact.get(&(u32::MAX, None)), None);
    }

    #[test]
    fn packed_freshness_keeps_unavailable_identities_across_word_boundaries() {
        let ordinary = (0..130)
            .map(|value| ((value, None), value % 3 == 0))
            .collect::<BTreeMap<_, _>>();
        let mut compact = ordinary
            .iter()
            .map(|(&key, &value)| (key, value))
            .collect::<State>();
        let original = compact.clone();
        let mut expanded = ordinary.clone();
        for value in [0, 63, 64, 127, 128, 129] {
            for field in [Some(0), Some(3)] {
                let key = (value, field);
                compact.insert(key, false);
                expanded.insert(key, false);
            }
        }
        exact(&compact, &expanded);
        exact(&original, &ordinary);
        compact.clear_where(|_| true);
        assert_eq!(compact.keys().count(), expanded.len());
        assert!(compact.keys().all(|key| compact.get(key) == Some(false)));
        assert!(
            compact
                .keys()
                .filter(|key| key.0 == 64)
                .any(|key| !compact.get(key).unwrap())
        );
    }

    #[test]
    fn packed_freshness_matches_ordered_maps_through_forks_and_path_meets() {
        let initial = (0..140)
            .map(|value| ((value, None), value % 5 == 0))
            .collect::<BTreeMap<_, _>>();
        let packed = initial
            .iter()
            .map(|(&key, &value)| (key, value))
            .collect::<State>();
        let mut compact = vec![packed; 7];
        let mut ordinary = vec![initial; 7];
        let mut seed = 719_021u64;
        for step in 0..4096 {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let target = (seed as usize >> 8) % compact.len();
            let source = (target + 1 + (seed as usize >> 16) % 6) % compact.len();
            let key = (
                ((seed >> 24) % 190) as u32,
                ((seed >> 40) % 4 != 0).then_some(((seed >> 44) % 4) as usize),
            );
            let value = seed & 1 != 0;
            match step % 6 {
                0 => {
                    compact[target].insert(key, value);
                    ordinary[target].insert(key, value);
                }
                1 => {
                    compact[target] = compact[source].clone();
                    ordinary[target] = ordinary[source].clone();
                }
                2 => {
                    let other = compact[source].clone();
                    let before = ordinary[target].clone();
                    let proven = ordinary[source].clone();
                    for (key, fresh) in &mut ordinary[target] {
                        *fresh &= proven.get(key).copied().unwrap_or(false);
                    }
                    assert_eq!(compact[target].meet(&other), before != ordinary[target]);
                }
                3 => {
                    let other = compact[source].clone();
                    compact[target].strengthen(&other);
                    let proven = ordinary[source].clone();
                    for (key, fresh) in &mut ordinary[target] {
                        *fresh |= proven.get(key).copied().unwrap_or(false);
                    }
                }
                4 => {
                    compact[target].clear_where(|candidate| candidate.0 % 7 == key.0 % 7);
                    for (candidate, fresh) in &mut ordinary[target] {
                        if candidate.0 % 7 == key.0 % 7 {
                            *fresh = false;
                        }
                    }
                }
                _ => {
                    compact[target].extend([(key, value), (key, !value)]);
                    ordinary[target].extend([(key, value), (key, !value)]);
                }
            }
            exact(&compact[target], &ordinary[target]);
            exact(&compact[source], &ordinary[source]);
        }
    }
}
