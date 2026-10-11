//! Library count tables with key/counter widths chosen from admitted bounds.
use hashbrown::HashMap;
type FastMap<K, V> = HashMap<K, V, fxhash::FxBuildHasher>;
type StdMap<K, V> = std::collections::HashMap<K, V, fxhash::FxBuildHasher>;
pub(super) enum Counts {
    Full(HashMap<u64, u64>),
    Small8(FastMap<u8, u32>),
    Small16(FastMap<u16, u32>),
    Small32(FastMap<u32, u32>),
    Small64(FastMap<u64, u32>),
    Wide(FastMap<u64, u64>),
    Std8(StdMap<u8, u32>),
    Std16(StdMap<u16, u32>),
    Std32(StdMap<u32, u32>),
    Std64(StdMap<u64, u32>),
    StdWide(StdMap<u64, u64>),
}
impl Counts {
    pub fn new(bits: u32, total: usize, compact: bool, standard: bool) -> Self {
        if !compact {
            return Self::Full(HashMap::new());
        }
        if standard {
            if total > u32::MAX as usize {
                return Self::StdWide(StdMap::default());
            }
            return match bits {
                0..=8 => Self::Std8(StdMap::default()),
                9..=16 => Self::Std16(StdMap::default()),
                17..=32 => Self::Std32(StdMap::default()),
                _ => Self::Std64(StdMap::default()),
            };
        }
        if total > u32::MAX as usize {
            return Self::Wide(FastMap::default());
        }
        match bits {
            0..=8 => Self::Small8(FastMap::default()),
            9..=16 => Self::Small16(FastMap::default()),
            17..=32 => Self::Small32(FastMap::default()),
            _ => Self::Small64(FastMap::default()),
        }
    }
    // Keep the table dispatch with its batch caller so unrelated resident
    // instruction growth cannot change the hot update call's inline decision.
    #[inline(always)]
    pub fn increment(&mut self, keys: &[u64]) {
        macro_rules! increment {
            ($map:expr, $key:ty) => {
                for &key in keys {
                    debug_assert!(key <= <$key>::MAX as u64);
                    *$map.entry(key as $key).or_insert(0) += 1;
                }
            };
        }
        // The compiled mask bounds every key. The admitted total bounds each
        // count and all merged counts; larger inputs select u64 counters.
        match self {
            Self::Full(map) => increment!(map, u64),
            Self::Small8(map) => increment!(map, u8),
            Self::Small16(map) => increment!(map, u16),
            Self::Small32(map) => increment!(map, u32),
            Self::Small64(map) => increment!(map, u64),
            Self::Wide(map) => increment!(map, u64),
            Self::Std8(map) => increment!(map, u8),
            Self::Std16(map) => increment!(map, u16),
            Self::Std32(map) => increment!(map, u32),
            Self::Std64(map) => increment!(map, u64),
            Self::StdWide(map) => increment!(map, u64),
        }
    }
    pub fn rows(&self) -> Vec<(u64, u64)> {
        macro_rules! rows {
            ($map:expr) => {
                $map.iter().map(|(&k, &v)| (k as u64, v as u64)).collect()
            };
        }
        match self {
            Self::Full(map) => rows!(map),
            Self::Small8(map) => rows!(map),
            Self::Small16(map) => rows!(map),
            Self::Small32(map) => rows!(map),
            Self::Small64(map) => rows!(map),
            Self::Wide(map) => rows!(map),
            Self::Std8(map) => rows!(map),
            Self::Std16(map) => rows!(map),
            Self::Std32(map) => rows!(map),
            Self::Std64(map) => rows!(map),
            Self::StdWide(map) => rows!(map),
        }
    }
    pub fn get(&self, key: u64) -> u64 {
        macro_rules! get {
            ($map:expr, $key:ty) => {
                if key > <$key>::MAX as u64 {
                    0
                } else {
                    $map.get(&(key as $key)).copied().unwrap_or(0) as u64
                }
            };
        }
        match self {
            Self::Full(map) => get!(map, u64),
            Self::Small8(map) => get!(map, u8),
            Self::Small16(map) => get!(map, u16),
            Self::Small32(map) => get!(map, u32),
            Self::Small64(map) => get!(map, u64),
            Self::Wide(map) => get!(map, u64),
            Self::Std8(map) => get!(map, u8),
            Self::Std16(map) => get!(map, u16),
            Self::Std32(map) => get!(map, u32),
            Self::Std64(map) => get!(map, u64),
            Self::StdWide(map) => get!(map, u64),
        }
    }
    pub fn merge(&mut self, other: Self) {
        macro_rules! merge {
            ($a:expr, $b:expr) => {
                for (key, count) in $b {
                    *$a.entry(key).or_insert(0) += count;
                }
            };
        }
        match (self, other) {
            (Self::Full(a), Self::Full(b)) => merge!(a, b),
            (Self::Small8(a), Self::Small8(b)) => merge!(a, b),
            (Self::Small16(a), Self::Small16(b)) => merge!(a, b),
            (Self::Small32(a), Self::Small32(b)) => merge!(a, b),
            (Self::Small64(a), Self::Small64(b)) => merge!(a, b),
            (Self::Wide(a), Self::Wide(b)) => merge!(a, b),
            (Self::Std8(a), Self::Std8(b)) => merge!(a, b),
            (Self::Std16(a), Self::Std16(b)) => merge!(a, b),
            (Self::Std32(a), Self::Std32(b)) => merge!(a, b),
            (Self::Std64(a), Self::Std64(b)) => merge!(a, b),
            (Self::StdWide(a), Self::StdWide(b)) => merge!(a, b),
            _ => panic!("partition count table shapes differ"),
        }
    }
}
