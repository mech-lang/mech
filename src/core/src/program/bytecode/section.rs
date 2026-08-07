pub const BYTECODE_SECTION_ENTRY_SIZE: usize = 32;
pub const BYTECODE_SECTION_COUNT: usize = 7;
pub const BYTECODE_SECTION_TABLE_OFFSET: u64 = 64;
pub const BYTECODE_CONTENT_OFFSET: u64 = 288;

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
    ];

    pub fn from_u16(value: u16) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| *kind as u16 == value)
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
