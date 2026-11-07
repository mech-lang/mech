//! Machine-backend association, never a source/layout permission.
use sha2::{Digest, Sha256};

/// Stored only for the distinct LLVM image family. The maintained Cranelift
/// descriptor has no extra record or changed identity/serialization.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct MachineBackend {
    kind: u64,
    revision: u64,
    source_ir: [u8; 32],
    options: [u8; 32],
    toolchain: [u8; 32],
    machine_object: [u8; 32],
}

impl MachineBackend {
    #[cfg(any(feature = "canonical-native-program", test))]
    pub(in crate::native_artifact) fn llvm(
        source_ir: [u8; 32],
        toolchain: [u8; 32],
        machine_object: [u8; 32],
    ) -> Result<Self, String> {
        let value = Self {
            kind: 1,
            revision: 1,
            source_ir,
            options: Self::options(),
            toolchain,
            machine_object,
        };
        value.validate()?;
        Ok(value)
    }
    fn options() -> [u8; 32] {
        Sha256::digest(b"mech.llvm-aarch64-apple-native.v1\0arm64-apple-macosx26.0.0\0apple-m3\0default<O3>\0O3\0pic\0global-isel=0\0machine-outliner=never\0frame-pointer=all\0uwtable\0align=128\0noinline\0ld-r-code-canonicalization\0metadata=__TEXT,__mech_meta\0").into()
    }
    pub(in crate::native_artifact) fn validate(&self) -> Result<(), String> {
        if self.kind != 1
            || self.revision != 1
            || self.options != Self::options()
            || self.source_ir == [0; 32]
            || self.toolchain == [0; 32]
            || self.machine_object == [0; 32]
        {
            return Err("unsupported native machine-backend record".into());
        }
        Ok(())
    }
    pub(in crate::native_artifact) fn identity(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(b"mech.native-machine-backend-identity.v1\0");
        hash.update(self.kind.to_le_bytes());
        hash.update(self.revision.to_le_bytes());
        for value in [
            &self.source_ir,
            &self.options,
            &self.toolchain,
            &self.machine_object,
        ] {
            hash.update(value);
        }
        hash.finalize().into()
    }
    pub(in crate::native_artifact) fn source_ir(&self) -> &[u8; 32] {
        &self.source_ir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn llvm_backend_identity_binds_each_machine_and_source_field() {
        let original = MachineBackend::llvm([1; 32], [2; 32], [3; 32]).unwrap();
        for value in [
            MachineBackend::llvm([9; 32], [2; 32], [3; 32]).unwrap(),
            MachineBackend::llvm([1; 32], [9; 32], [3; 32]).unwrap(),
            MachineBackend::llvm([1; 32], [2; 32], [9; 32]).unwrap(),
        ] {
            assert_ne!(original.identity(), value.identity());
        }
        let mut value = original.clone();
        value.kind += 1;
        assert!(value.validate().is_err());
        value = original.clone();
        value.revision += 1;
        assert!(value.validate().is_err());
        value = original;
        value.options[0] ^= 1;
        assert!(value.validate().is_err());
        assert!(MachineBackend::llvm([1; 32], [0; 32], [3; 32]).is_err());
    }
}
