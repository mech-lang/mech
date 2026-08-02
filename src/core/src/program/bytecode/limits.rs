#[derive(Clone, Debug)]
pub struct BytecodeReadLimits {
    pub max_file_bytes: usize,
    pub max_registers: u32,
    pub max_instructions: u32,
    pub max_types: u32,
    pub max_constants: u32,
    pub max_symbols: u32,
    pub max_dictionary_entries: u32,
    pub max_dictionary_bytes: usize,
    pub max_requirements: u32,
    pub max_variadic_arguments: u32,
}

impl Default for BytecodeReadLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: 67_108_864,
            max_registers: 1_000_000,
            max_instructions: 1_000_000,
            max_types: 100_000,
            max_constants: 1_000_000,
            max_symbols: 1_000_000,
            max_dictionary_entries: 1_000_000,
            max_dictionary_bytes: 16_777_216,
            max_requirements: 10_000,
            max_variadic_arguments: 65_536,
        }
    }
}
