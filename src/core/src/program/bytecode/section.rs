#[cfg(feature = "no_std")]
use alloc::vec::Vec;
#[cfg(not(feature = "no_std"))]
use std::vec::Vec;

pub const BYTECODE_SECTION_ENTRY_SIZE: usize = 32;
pub const BYTECODE_SECTION_COUNT: usize = 17;
pub const BYTECODE_SECTION_TABLE_OFFSET: u64 = 64;
pub const BYTECODE_CONTENT_OFFSET: u64 = 608;

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BytecodeSectionKind {
    Types = 1,
    ConstantTable = 2,
    ConstantBlob = 3,
    Symbols = 4,
    Instructions = 5,
    Dictionary = 6,
    ApplicationRequirements = 7,
    ArtifactSchemas = 8,
    ArtifactConstants = 9,
    ArtifactInputs = 10,
    ArtifactSlots = 11,
    ArtifactProducers = 12,
    ArtifactNodes = 13,
    ArtifactBindings = 14,
    ArtifactOutputs = 15,
    ArtifactIntegrityConstraints = 16,
    ArtifactOperations = 17,
}

impl BytecodeSectionKind {
    pub const ALL: [Self; BYTECODE_SECTION_COUNT] = [
        Self::Types,
        Self::ConstantTable,
        Self::ConstantBlob,
        Self::Symbols,
        Self::Instructions,
        Self::Dictionary,
        Self::ApplicationRequirements,
        Self::ArtifactSchemas,
        Self::ArtifactConstants,
        Self::ArtifactInputs,
        Self::ArtifactSlots,
        Self::ArtifactProducers,
        Self::ArtifactNodes,
        Self::ArtifactBindings,
        Self::ArtifactOutputs,
        Self::ArtifactIntegrityConstraints,
        Self::ArtifactOperations,
    ];

    pub fn from_u16(value: u16) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| *kind as u16 == value)
    }
}

/// Canonical semantic-program payloads carried by bytecode v1.
///
/// The core bytecode reader owns section framing and limits. `mech-engine`
/// owns the typed `ProgramArtifact` codec and validates every decoded field
/// through `ProgramArtifactDraft::finalize`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BytecodeArtifactSections {
    pub schemas: Vec<u8>,
    pub constants: Vec<u8>,
    pub inputs: Vec<u8>,
    pub slots: Vec<u8>,
    pub producers: Vec<u8>,
    pub nodes: Vec<u8>,
    pub bindings: Vec<u8>,
    pub outputs: Vec<u8>,
    pub integrity_constraints: Vec<u8>,
    pub operations: Vec<u8>,
}

impl BytecodeArtifactSections {
    pub fn is_empty(&self) -> bool {
        self.schemas.is_empty()
            && self.constants.is_empty()
            && self.inputs.is_empty()
            && self.slots.is_empty()
            && self.producers.is_empty()
            && self.nodes.is_empty()
            && self.bindings.is_empty()
            && self.outputs.is_empty()
            && self.integrity_constraints.is_empty()
            && self.operations.is_empty()
    }

    pub(crate) fn ordered(&self) -> [&[u8]; 10] {
        [
            &self.schemas,
            &self.constants,
            &self.inputs,
            &self.slots,
            &self.producers,
            &self.nodes,
            &self.bindings,
            &self.outputs,
            &self.integrity_constraints,
            &self.operations,
        ]
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BytecodeSectionEntry {
    pub kind: BytecodeSectionKind,
    pub flags: u16,
    pub item_count: u32,
    pub offset: u64,
    pub length: u64,
    pub reserved: u64,
}
