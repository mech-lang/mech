//! Temporary dimension identity remapping. Consecutive renaming stays inline;
//! other identities retain bounded dense and sparse storage.
use mech_core::DimensionParameterId as Id;
use std::collections::BTreeMap;

const MAX_DENSE: usize = 65_536;
#[derive(Clone, Copy)]
struct Consecutive {
    from: u32,
    to: u32,
    len: usize,
}
#[derive(Default)]
pub(super) struct Remapping {
    consecutive: Option<Consecutive>,
    dense: Vec<Option<Id>>,
    sparse: BTreeMap<Id, Id>,
    len: usize,
}
impl Remapping {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn get(&self, id: &Id) -> Option<Id> {
        if let Some(range) = self.consecutive {
            let offset = id.get().checked_sub(range.from)?;
            // Appending checks both actual IDs before extending the range;
            // every represented target therefore fits u32, including MAX.
            return ((offset as usize) < range.len).then(|| Id::new(range.to + offset));
        }
        self.dense
            .get(id.get() as usize)
            .and_then(Option::as_ref)
            .or_else(|| self.sparse.get(id))
            .copied()
    }
    pub fn at(&self, id: &Id) -> Id {
        self.get(id).expect("no entry found for key")
    }
    pub fn insert(&mut self, id: Id, value: Id) -> Option<Id> {
        if self.len == 0 {
            self.consecutive = Some(Consecutive {
                from: id.get(),
                to: value.get(),
                len: 1,
            });
            self.len = 1;
            return None;
        }
        if let Some(range) = self.consecutive {
            // Use wider comparisons so a range ending at MAX cannot wrap and
            // accidentally admit zero as its next key or target.
            if u64::from(id.get()) == u64::from(range.from) + range.len as u64
                && u64::from(value.get()) == u64::from(range.to) + range.len as u64
            {
                self.consecutive.as_mut().unwrap().len += 1;
                self.len += 1;
                return None;
            }
            if self.get(&id) == Some(value) {
                return Some(value);
            }
            self.materialize();
        }
        self.insert_stored(id, value)
    }
    fn materialize(&mut self) {
        let range = self.consecutive.take().unwrap();
        self.len = 0;
        for offset in 0..range.len {
            self.insert_stored(
                Id::new((u64::from(range.from) + offset as u64) as u32),
                Id::new((u64::from(range.to) + offset as u64) as u32),
            );
        }
    }
    fn insert_stored(&mut self, id: Id, value: Id) -> Option<Id> {
        let index = id.get() as usize;
        let previous = if index < MAX_DENSE && index <= self.dense.len() + 64 {
            if index >= self.dense.len() {
                self.dense.resize(index + 1, None);
            }
            self.dense[index]
                .replace(value)
                .or_else(|| self.sparse.remove(&id))
        } else {
            self.sparse.insert(id, value)
        };
        if previous.is_none() {
            self.len += 1;
        }
        previous
    }
}
impl FromIterator<(Id, Id)> for Remapping {
    fn from_iter<T: IntoIterator<Item = (Id, Id)>>(items: T) -> Self {
        let mut result = Self::new();
        for (id, value) in items {
            result.insert(id, value);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dense_sparse_and_duplicate_remaps_match_ordered_map() {
        let mut actual = Remapping::new();
        let mut expected = BTreeMap::new();
        let mut random = 1_u64;
        for step in 0..50_000_u32 {
            random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
            let key = match step % 5 {
                0 => step / 5,
                1 => (random >> 32) as u32 % 2048,
                2 => u32::MAX - (random >> 32) as u32 % 16,
                3 => 65_500 + (random >> 32) as u32 % 100,
                _ => (random >> 32) as u32,
            };
            let id = Id::new(key);
            let value = Id::new(step);
            assert_eq!(actual.insert(id, value), expected.insert(id, value));
            assert_eq!(actual.get(&id), expected.get(&id).copied());
            assert_eq!(actual.at(&id), expected[&id]);
            assert_eq!(actual.len(), expected.len());
            assert!(actual.dense.len() <= MAX_DENSE);
        }
        for (id, value) in expected {
            assert_eq!(actual.get(&id), Some(value));
        }
        assert_eq!(actual.get(&Id::new(32_000)), None);
    }

    #[test]
    fn growing_prefix_migrates_sparse_entries_without_counting_them_twice() {
        let mut map = Remapping::new();
        assert_eq!(map.insert(Id::new(128), Id::new(7)), None);
        assert_eq!(map.dense.len(), 0);
        for key in 0..128 {
            map.insert(Id::new(key), Id::new(key + 1));
        }
        assert_eq!(map.insert(Id::new(128), Id::new(9)), Some(Id::new(7)));
        assert_eq!(map.len(), 129);
        assert!(!map.sparse.contains_key(&Id::new(128)));
        assert_eq!(map.at(&Id::new(128)), Id::new(9));
    }

    #[test]
    fn iterator_preserves_last_duplicate_and_extreme_identity() {
        let map = [
            (Id::new(0), Id::new(1)),
            (Id::new(u32::MAX), Id::new(8)),
            (Id::new(0), Id::new(2)),
        ]
        .into_iter()
        .collect::<Remapping>();
        assert_eq!(map.len(), 2);
        assert_eq!(map.at(&Id::new(0)), Id::new(2));
        assert_eq!(map.at(&Id::new(u32::MAX)), Id::new(8));
        assert_eq!(map.dense.len(), 1);
    }

    #[test]
    fn consecutive_translation_stays_inline_past_the_dense_storage_limit() {
        let mut map = Remapping::new();
        for key in 0..200_000 {
            let id = Id::new(key);
            assert_eq!(map.get(&id), None, "future identities are not mapped early");
            assert_eq!(map.insert(id, Id::new(key + 17)), None);
            assert_eq!(map.get(&id), Some(Id::new(key + 17)));
        }
        assert_eq!(map.len(), 200_000);
        assert_eq!(map.get(&Id::new(0)), Some(Id::new(17)));
        assert_eq!(map.get(&Id::new(200_000)), None);
        assert!(map.dense.is_empty() && map.sparse.is_empty());
        assert_eq!(map.insert(Id::new(123), Id::new(140)), Some(Id::new(140)));
        assert!(map.dense.is_empty() && map.sparse.is_empty());
        assert_eq!(map.insert(Id::new(123), Id::new(9)), Some(Id::new(140)));
        assert_eq!(map.len(), 200_000);
        for key in 0..200_000 {
            assert_eq!(
                map.get(&Id::new(key)),
                Some(Id::new(if key == 123 { 9 } else { key + 17 }))
            );
        }
        assert!(map.dense.len() <= MAX_DENSE);
    }

    #[test]
    fn gaps_replacements_nonzero_starts_and_overflow_boundaries_match_ordered_map() {
        for from in [0, 91, 65_530, 1_000_000, u32::MAX - 31] {
            for to in [0, 37, u32::MAX - 31] {
                let mut map = Remapping::new();
                let mut reference = BTreeMap::new();
                for (key, value) in (0..32).map(|n| (from + n, to + n)).chain([
                    (from + 7, to + 7),
                    (from + 7, 42),
                    (0, u32::MAX),
                    (u32::MAX, 0),
                    (from, to),
                ]) {
                    let (id, target) = (Id::new(key), Id::new(value));
                    assert_eq!(map.insert(id, target), reference.insert(id, target));
                    assert_eq!(map.len(), reference.len());
                    for (&id, &target) in &reference {
                        assert_eq!(map.get(&id), Some(target));
                    }
                    assert_eq!(map.get(&Id::new(123_456_789)), None);
                }
            }
        }
        let mut map: Remapping = [(Id::new(1), Id::new(u32::MAX))].into_iter().collect();
        assert_eq!(map.insert(Id::new(2), Id::new(0)), None);
        assert_eq!(map.get(&Id::new(1)), Some(Id::new(u32::MAX)));
        assert_eq!(map.get(&Id::new(2)), Some(Id::new(0)));
    }
}
