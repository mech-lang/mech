//! Temporary dimension identity remapping. Dense prefixes use direct entries;
//! sparse/high identities retain ordered-map storage without a large arena.
use mech_core::DimensionParameterId as Id;
use std::{collections::BTreeMap, ops::Index};

const MAX_DENSE: usize = 65_536;
#[derive(Default)]
pub(super) struct Remapping {
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
    pub fn get(&self, id: &Id) -> Option<&Id> {
        self.dense
            .get(id.get() as usize)
            .and_then(Option::as_ref)
            .or_else(|| self.sparse.get(id))
    }
    pub fn insert(&mut self, id: Id, value: Id) -> Option<Id> {
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
impl Index<&Id> for Remapping {
    type Output = Id;
    fn index(&self, id: &Id) -> &Id {
        self.get(id).expect("no entry found for key")
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
            assert_eq!(actual.get(&id), expected.get(&id));
            assert_eq!(actual[&id], expected[&id]);
            assert_eq!(actual.len(), expected.len());
            assert!(actual.dense.len() <= MAX_DENSE);
        }
        for (id, value) in expected {
            assert_eq!(actual.get(&id), Some(&value));
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
        assert_eq!(map[&Id::new(128)], Id::new(9));
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
        assert_eq!(map[&Id::new(0)], Id::new(2));
        assert_eq!(map[&Id::new(u32::MAX)], Id::new(8));
        assert_eq!(map.dense.len(), 1);
    }
}
