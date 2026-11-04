//! Shared checked numeric representation for leaves, parent pieces, and phases.
//! Source membership, ownership authority and prices are independently derived;
//! these records describe operations and word homes, never a second program.
use super::readonly_numeric::ResultRecipe;
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Array {
    pub field: usize,
    pub width: u8,
    /// Exact selected site whose original factory validates the entry view.
    pub witness: usize,
    pub writable: bool,
    pub appends: bool,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct FrameWords {
    pub value: u32,
    /// Relative native scalar lane, or None for an invariant managed field.
    pub fields: Vec<Option<usize>>,
    pub arrays: Vec<Option<usize>>,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) enum Operation {
    /// The summary edge is descriptive only; execute the original child here.
    Fallback,
    Frame,
    Aggregate,
    ArrayCopy,
    Length {
        array: usize,
        output: u32,
    },
    Read {
        array: usize,
        index: u32,
        output: u32,
    },
    Write {
        array: usize,
        index: u32,
        incoming: u32,
    },
    Append {
        array: usize,
        incoming: u32,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) enum ExitKind {
    Unsupported,
    ChildInitialize { iteration: u32 },
    ChildHeader { iteration: u32 },
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Exit {
    pub from: usize,
    pub to: usize,
    pub frames: Vec<u32>,
    pub aliases: Vec<(u32, usize)>,
    /// All live fields have been exported, and the old root visit is dead.
    pub retire_carrier: bool,
    pub kind: ExitKind,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Plan {
    pub arrays: Vec<Array>,
    pub frames: Vec<FrameWords>,
    pub actions: Vec<(usize, Operation)>,
    pub sites: Vec<usize>,
    pub prepaid_sites: Vec<usize>,
    pub scalar_words: usize,
    pub words: usize,
    pub result: ResultRecipe,
    pub phase: Option<super::numeric_phase::Schedule>,
    pub exits: Vec<Exit>,
    /// Source-local values that cannot observe their previous root traversal.
    pub fresh_values: Vec<u32>,
}
impl Plan {
    /// Only current input lanes cross an initial entry. Internal frame lanes
    /// are defined by the selected source actions, not by the helper's scratch
    /// bank. Keep the full bank's admission and first-change transport intact.
    #[cfg(feature = "canonical-native-program")]
    pub fn entry_words(&self, carrier: u32) -> Result<Vec<usize>, &'static str> {
        let mut words = self
            .fields(carrier)
            .ok_or("numeric entry lacks its carrier fields")?
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        for array in 0..self.arrays.len() {
            words.extend(self.view(array)..self.view(array) + 4);
            if self.writes() {
                words.push(self.private_flag(array));
            }
            words.extend(self.capacity(array));
        }
        if let Some(phase) = &self.phase {
            for debit in &phase.loops {
                words.push(debit.counter);
                words.extend(debit.first_visit);
            }
        }
        words.sort_unstable();
        words.dedup();
        if words.last().is_some_and(|word| *word >= self.words) {
            return Err("numeric entry input is outside its admitted word bank");
        }
        Ok(words)
    }

    pub fn action(&self, site: usize) -> Option<&Operation> {
        self.actions
            .iter()
            .find(|(s, _)| *s == site)
            .map(|(_, action)| action)
    }
    pub fn fields(&self, value: u32) -> Option<&[Option<usize>]> {
        self.frames
            .iter()
            .find(|f| f.value == value)
            .map(|f| f.fields.as_slice())
    }
    pub fn view(&self, array: usize) -> usize {
        self.scalar_words + array * 4
    }
    pub fn writes(&self) -> bool {
        self.arrays.iter().any(|array| array.writable)
    }
    pub fn private_flag(&self, array: usize) -> usize {
        self.scalar_words + self.arrays.len() * 4 + array
    }
    pub fn capacity(&self, array: usize) -> Option<usize> {
        self.arrays[array].appends.then(|| {
            self.scalar_words
                + self.arrays.len() * 5
                + self.arrays[..array].iter().filter(|a| a.appends).count()
        })
    }
}
