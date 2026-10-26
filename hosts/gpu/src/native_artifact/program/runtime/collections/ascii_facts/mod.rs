//! One same-entry representation fact, never payload or permission ownership.
use crate::native_artifact::program::runtime::{NativeProgramExecutionError as Error, invalid};
use mech_core::snapshot::FrozenSnapshotIdentity;
use mech_core::{Value, ValueData};

struct Fact {
    owner: FrozenSnapshotIdentity,
    ascii_without_crlf: bool,
}

#[derive(Default)]
pub(in crate::native_artifact::program::runtime::collections) struct Cache {
    fact: Option<Fact>,
}

impl Cache {
    /// The owning collection state admits the inline slot AND the complete
    /// weak Arc shell before constructing this cache. No payload is retained.
    pub(in crate::native_artifact::program::runtime::collections) const fn metadata_bytes() -> u64 {
        core::mem::size_of::<Self>() as u64 + Value::frozen_storage_identity_allocation_bytes()
    }

    /// Caller first pays its complete existing representation-scan bound,
    /// even on a hit. Fresh source kind, owner, geometry, quotas and execution
    /// phase checks remain the caller's responsibility on EVERY invocation.
    pub(in crate::native_artifact::program::runtime::collections) fn inspect(
        &mut self,
        value: &Value,
    ) -> Result<bool, Error> {
        let ValueData::String(_) = value.data() else {
            return Err(invalid("native representation fact requires a String"));
        };
        if let Some(fact) = &self.fact
            && fact.owner.matches(value)
        {
            #[cfg(all(test, feature = "canonical-native-program"))]
            tests::hit();
            return Ok(fact.ascii_without_crlf);
        }
        let ascii_without_crlf = value
            .string_ascii_without_crlf()
            .ok_or_else(|| invalid("native representation fact requires a String"))?;
        #[cfg(all(test, feature = "canonical-native-program"))]
        tests::scanned();
        // Release the old weak shell before retaining a replacement. The
        // genuine current source is still strongly owned by the caller.
        self.fact = None;
        self.fact = Some(Fact {
            owner: value.frozen_storage_identity(),
            ascii_without_crlf,
        });
        Ok(ascii_without_crlf)
    }
}

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "tests.rs"]
pub(in crate::native_artifact::program::runtime::collections::ascii_facts) mod tests;
