pub const BYTECODE_MAGIC: [u8; 4] = *b"MECH";
pub const BYTECODE_VERSION: u16 = 1;
pub const BYTECODE_HEADER_SIZE: u16 = 64;

/// The Mech language/runtime ABI accepted by bytecode v1 readers and writers.
///
/// This is intentionally independent of any individual crate or distribution
/// package version. Changing bytecode compatibility is an explicit ABI decision.
pub const MECH_LANGUAGE_RUNTIME_ABI_VERSION: (u16, u16, u16) = (0, 3, 5);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BytecodeHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub header_size: u16,
    pub mech_major: u16,
    pub mech_minor: u16,
    pub mech_patch: u16,
    pub flags: u16,
    pub register_count: u32,
    pub instruction_count: u32,
    pub section_count: u16,
    pub reserved0: u16,
    pub section_table_offset: u64,
    pub file_len: u64,
    pub checksum_offset: u64,
    pub reserved: [u8; 12],
}
