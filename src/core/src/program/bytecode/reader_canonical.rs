//! Strict canonical-artifact admission over the shared bytecode-v1 envelope.
//! This is not another wire format or semantic artifact decoder. It omits only
//! legacy execution payload decoding, because any such payload is prohibited.
use super::*;

#[derive(Debug)]
pub enum CanonicalArtifactBytecodeError {
    CoreBytecode(MechError),
    MissingArtifactSections,
    ParallelLegacyExecutionAuthority { sections: Vec<&'static str> },
}

impl From<MechError> for CanonicalArtifactBytecodeError {
    fn from(error: MechError) -> Self {
        Self::CoreBytecode(error)
    }
}

/// Structurally checked canonical-artifact sections and their outer authority
/// table. The engine must still decode/finalize every semantic section and
/// require exact agreement with the artifact's embedded requirements.
#[derive(Debug)]
pub struct CanonicalArtifactBytecode {
    pub requirements: Vec<ApplicationRequirement>,
    pub artifact: BytecodeArtifactSections,
}

impl CanonicalArtifactBytecode {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CanonicalArtifactBytecodeError> {
        Self::from_bytes_with_limits(bytes, BytecodeReadLimits::default())
    }

    pub fn from_bytes_with_limits(
        bytes: &[u8],
        limits: BytecodeReadLimits,
    ) -> Result<Self, CanonicalArtifactBytecodeError> {
        // SAME file/header/version/checksum/directory/alignment validation as
        // ParsedProgram. No unchecked section or alternate envelope is accepted.
        let (header, sections) = parse_envelope(bytes, &limits)?;
        let requirements = parse_application_requirements(bytes, &sections, &limits)?;
        let artifact = parse_artifact_sections(bytes, &sections, &limits)?;
        if artifact.is_empty() {
            return Err(CanonicalArtifactBytecodeError::MissingArtifactSections);
        }
        let mut prohibited = Vec::new();
        for (present, name) in [
            (header.register_count != 0, "registers"),
            (header.instruction_count != 0, "instruction header"),
        ] {
            if present {
                prohibited.push(name);
            }
        }
        for (kind, name) in [
            (BytecodeSectionKind::Types, "types"),
            (BytecodeSectionKind::ConstantTable, "constants"),
            (BytecodeSectionKind::ConstantBlob, "constant blob"),
            (BytecodeSectionKind::Symbols, "symbols"),
            (BytecodeSectionKind::Instructions, "instructions"),
            (BytecodeSectionKind::Dictionary, "dictionary"),
        ] {
            // Both are required: a zero item count cannot conceal payload bytes,
            // and an empty byte range cannot conceal a nonzero declared count.
            let section = sections
                .iter()
                .find(|section| section.kind == kind)
                .unwrap();
            if section.item_count != 0 || section.length != 0 {
                prohibited.push(name);
            }
        }
        if !prohibited.is_empty() {
            // Deliberately reject rather than interpret malformed legacy data.
            // The ordinary ParsedProgram entry retains its complete decoding.
            return Err(
                CanonicalArtifactBytecodeError::ParallelLegacyExecutionAuthority {
                    sections: prohibited,
                },
            );
        }
        Ok(Self {
            requirements,
            artifact,
        })
    }
}

#[cfg(test)]
#[path = "reader_canonical_tests.rs"]
mod tests;
