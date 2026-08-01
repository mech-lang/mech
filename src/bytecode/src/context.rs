use crate::{BufferPositionMismatchError, FinalBufferLengthMismatchError};
use byteorder::{LittleEndian, WriteBytesExt};
use mech_core::*;
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Write};

#[derive(Debug)]
pub struct CompileCtx {
    reg_map: HashMap<usize, Register>,
    initialized_ptrs: HashSet<usize>,
    symbols: HashMap<u64, Register>,
    symbol_ptrs: HashMap<u64, usize>,
    dictionary: HashMap<u64, String>,
    mutable_symbols: HashSet<u64>,
    types: TypeSection,
    features: HashSet<FeatureFlag>,
    const_entries: Vec<ConstEntry>,
    const_blob: Vec<u8>,
    instrs: Vec<EncodedInstr>,
    next_reg: Register,
}

impl CompileCtx {
    pub fn new() -> Self {
        Self {
            reg_map: HashMap::new(),
            initialized_ptrs: HashSet::new(),
            symbols: HashMap::new(),
            mutable_symbols: HashSet::new(),
            dictionary: HashMap::new(),
            types: TypeSection::new(),
            symbol_ptrs: HashMap::new(),
            features: HashSet::new(),
            const_entries: Vec::new(),
            const_blob: Vec::new(),
            instrs: Vec::new(),
            next_reg: 0,
        }
    }

    pub fn clear(&mut self) {
        self.reg_map.clear();
        self.initialized_ptrs.clear();
        self.symbols.clear();
        self.dictionary.clear();
        self.mutable_symbols.clear();
        self.types = TypeSection::new();
        self.features.clear();
        self.const_entries.clear();
        self.const_blob.clear();
        self.instrs.clear();
        self.next_reg = 0;
    }

    pub fn compile(&mut self) -> MResult<Vec<u8>> {
        let header_size = ByteCodeHeader::HEADER_SIZE as u64;
        let feat_bytes_len = 4 + (self.features.len() as u64) * 8;
        let types_bytes_len = self.types.byte_len();
        let const_tbl_len = (self.const_entries.len() as u64) * ConstEntry::byte_len();
        let const_blob_len = self.const_blob.len() as u64;
        let symbols_len = (self.symbols.len() as u64) * 13;
        let instr_bytes_len = self
            .instrs
            .iter()
            .map(|instruction| instruction.byte_len())
            .sum();
        let dict_len = self
            .dictionary
            .values()
            .map(|name| name.len() as u64 + 12)
            .sum();

        let mut offset = header_size;
        let feature_off = offset;
        offset += feat_bytes_len;
        let types_off = offset;
        offset += types_bytes_len;
        let const_tbl_off = offset;
        offset += const_tbl_len;
        let const_blob_off = offset;
        offset += const_blob_len;
        let symbols_off = offset;
        offset += symbols_len;
        let instr_off = offset;
        offset += instr_bytes_len;
        let dict_off = offset;
        offset += dict_len;

        let file_len_before_trailer = offset;
        let full_file_len = file_len_before_trailer + 4;
        let header = ByteCodeHeader {
            magic: *b"MECH",
            version: 1,
            mech_ver: parse_version_to_u16(env!("CARGO_PKG_VERSION")).unwrap(),
            flags: 0,
            reg_count: self.next_reg,
            instr_count: self.instrs.len() as u32,
            feature_count: self.features.len() as u32,
            feature_off,
            types_count: self.types.entries.len() as u32,
            types_off,
            const_count: self.const_entries.len() as u32,
            const_tbl_off,
            const_tbl_len,
            const_blob_off,
            const_blob_len,
            symbols_len,
            symbols_off,
            instr_off,
            instr_len: instr_bytes_len,
            dict_len,
            dict_off,
            reserved: 0,
        };

        let mut buffer = Cursor::new(Vec::with_capacity(full_file_len as usize));
        header.write_to(&mut buffer)?;

        buffer.write_u32::<LittleEndian>(self.features.len() as u32)?;
        for feature in &self.features {
            buffer.write_u64::<LittleEndian>(feature.as_u64())?;
        }

        self.types.write_to(&mut buffer)?;

        for entry in &self.const_entries {
            entry.write_to(&mut buffer)?;
        }
        if !self.const_blob.is_empty() {
            buffer.write_all(&self.const_blob)?;
        }

        for (id, register) in &self.symbols {
            SymbolEntry::new(*id, self.mutable_symbols.contains(id), *register)
                .write_to(&mut buffer)?;
        }

        for instruction in &self.instrs {
            instruction.write_to(&mut buffer)?;
        }

        for (id, name) in &self.dictionary {
            DictEntry::new(*id, name).write_to(&mut buffer)?;
        }

        validate_buffer_position(buffer.position(), file_len_before_trailer)?;

        let checksum = crc32fast::hash(buffer.get_ref().as_slice());
        buffer.write_u32::<LittleEndian>(checksum)?;

        validate_final_buffer_length(buffer.position(), full_file_len)?;

        Ok(buffer.into_inner())
    }

    pub fn requirements(&self) -> &HashSet<FeatureFlag> {
        &self.features
    }

    fn alloc_register_for_ptr(&mut self, pointer: usize) -> Register {
        if let Some(&register) = self.reg_map.get(&pointer) {
            return register;
        }
        let register = self.next_reg;
        self.next_reg += 1;
        self.reg_map.insert(pointer, register);
        register
    }

    fn register_for_ptr_with_initialization_status(&mut self, pointer: usize) -> (Register, bool) {
        let register = self.alloc_register_for_ptr(pointer);
        let needs_initialization = self.initialized_ptrs.insert(pointer);
        (register, needs_initialization)
    }

    fn compile_const(&mut self, bytes: &[u8], value_kind: ValueKind) -> MResult<u32> {
        let type_id = self.types.get_or_intern(&value_kind);
        let align = value_kind.align();
        let next_blob_len = self.const_blob.len() as u64;
        let padded_offset = align_up(next_blob_len, align as u64);
        if padded_offset > next_blob_len {
            self.const_blob.resize(padded_offset as usize, 0);
        }
        self.features
            .insert(FeatureFlag::Builtin(value_kind.to_feature_kind()));
        let offset = self.const_blob.len() as u64;
        self.const_blob.extend_from_slice(bytes);
        let length = (self.const_blob.len() as u64) - offset;
        let entry = ConstEntry {
            type_id,
            enc: ConstEncoding::Inline,
            align: align as u8,
            flags: 0,
            reserved: 0,
            offset,
            length,
        };
        let const_id = self.const_entries.len() as u32;
        self.const_entries.push(entry);
        Ok(const_id)
    }
}

impl BytecodeCompilerContext for CompileCtx {
    fn register_for_ptr_with_initialization_status(&mut self, pointer: usize) -> (Register, bool) {
        CompileCtx::register_for_ptr_with_initialization_status(self, pointer)
    }

    fn compile_const(&mut self, bytes: &[u8], kind: ValueKind) -> MResult<u32> {
        CompileCtx::compile_const(self, bytes, kind)
    }

    fn define_symbol(&mut self, pointer: usize, register: Register, name: &str, mutable: bool) {
        let symbol_id = hash_str(name);
        self.symbols.insert(symbol_id, register);
        self.symbol_ptrs.insert(symbol_id, pointer);
        self.dictionary.insert(symbol_id, name.to_string());
        if mutable {
            self.mutable_symbols.insert(symbol_id);
        }
    }

    fn require(&mut self, requirement: FeatureFlag) {
        self.features.insert(requirement);
    }

    fn emit_const_load(&mut self, destination: Register, constant: u32) {
        self.instrs.push(EncodedInstr::ConstLoad {
            dst: destination,
            const_id: constant,
        });
    }

    fn emit_nullop(&mut self, function: u64, destination: Register) {
        self.instrs.push(EncodedInstr::NullOp {
            fxn_id: function,
            dst: destination,
        });
    }

    fn emit_unop(&mut self, function: u64, destination: Register, source: Register) {
        self.instrs.push(EncodedInstr::UnOp {
            fxn_id: function,
            dst: destination,
            src: source,
        });
    }

    fn emit_binop(&mut self, function: u64, destination: Register, lhs: Register, rhs: Register) {
        self.instrs.push(EncodedInstr::BinOp {
            fxn_id: function,
            dst: destination,
            lhs,
            rhs,
        });
    }

    fn emit_ternop(
        &mut self,
        function: u64,
        destination: Register,
        a: Register,
        b: Register,
        c: Register,
    ) {
        self.instrs.push(EncodedInstr::TernOp {
            fxn_id: function,
            dst: destination,
            a,
            b,
            c,
        });
    }

    fn emit_quadop(
        &mut self,
        function: u64,
        destination: Register,
        a: Register,
        b: Register,
        c: Register,
        d: Register,
    ) {
        self.instrs.push(EncodedInstr::QuadOp {
            fxn_id: function,
            dst: destination,
            a,
            b,
            c,
            d,
        });
    }

    fn emit_varop(&mut self, function: u64, destination: Register, arguments: Vec<Register>) {
        self.instrs.push(EncodedInstr::VarArg {
            fxn_id: function,
            dst: destination,
            args: arguments,
        });
    }

    fn emit_ret(&mut self, source: Register) {
        self.instrs.push(EncodedInstr::Ret { src: source });
    }
}

#[inline]
fn align_up(offset: u64, align: u64) -> u64 {
    if align == 0 {
        return offset;
    }
    ((offset + align - 1) / align) * align
}

fn validate_buffer_position(position: u64, expected: u64) -> MResult<()> {
    if position == expected {
        return Ok(());
    }
    Err(MechError::new(
        BufferPositionMismatchError {
            expected,
            got: position,
        },
        None,
    )
    .with_compiler_loc())
}

fn validate_final_buffer_length(length: u64, expected: u64) -> MResult<()> {
    if length == expected {
        return Ok(());
    }
    Err(MechError::new(
        FinalBufferLengthMismatchError {
            expected,
            got: length,
        },
        None,
    )
    .with_compiler_loc())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_context_initializes_pointer_register_once() {
        let mut context = CompileCtx::new();
        let pointer_a = 100usize;
        let pointer_b = 200usize;

        let (register_a, initializes_a) =
            context.register_for_ptr_with_initialization_status(pointer_a);
        assert!(initializes_a);

        let (register_a_again, initializes_a_again) =
            context.register_for_ptr_with_initialization_status(pointer_a);
        assert_eq!(register_a_again, register_a);
        assert!(!initializes_a_again);

        let (register_b, initializes_b) =
            context.register_for_ptr_with_initialization_status(pointer_b);
        assert_ne!(register_b, register_a);
        assert!(initializes_b);

        context.clear();

        let (register_a_after_clear, initializes_a_after_clear) =
            context.register_for_ptr_with_initialization_status(pointer_a);
        assert_eq!(register_a_after_clear, 0);
        assert!(initializes_a_after_clear);
    }

    #[test]
    fn pointer_register_scalar_initializes_once() {
        let mut context = CompileCtx::new();
        let context = &mut context;
        let scalar_a = Ref::new(42usize);
        let scalar_b = Ref::new(42usize);

        let register_a = compile_register_brrw!(scalar_a, context);
        let register_a_again = compile_register_brrw!(scalar_a, context);
        let register_b = compile_register_brrw!(scalar_b, context);

        assert_eq!(register_a_again, register_a);
        assert_ne!(register_b, register_a);
        assert_eq!(context.const_entries.len(), 2);
        let const_loads = context
            .instrs
            .iter()
            .filter_map(|instruction| match instruction {
                EncodedInstr::ConstLoad { dst, const_id } => Some((*dst, *const_id)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(const_loads, vec![(register_a, 0), (register_b, 1)]);
    }

    #[test]
    fn distinct_pointers_with_equal_values_receive_distinct_registers() {
        let value_a = 42u64;
        let value_b = 42u64;
        let mut context = CompileCtx::new();

        let value_a_address = std::ptr::from_ref(&value_a).addr();
        let value_b_address = std::ptr::from_ref(&value_b).addr();
        let (register_a, _) = context.register_for_ptr_with_initialization_status(value_a_address);
        let (register_b, _) = context.register_for_ptr_with_initialization_status(value_b_address);

        assert_ne!(value_a_address, value_b_address);
        assert_ne!(register_a, register_b);
    }

    #[test]
    fn symbol_and_mutability_metadata_survive_emission() {
        let mut context = CompileCtx::new();
        let pointer = 0x1234usize;
        let (register, _) = context.register_for_ptr_with_initialization_status(pointer);
        context.define_symbol(pointer, register, "answer", true);

        let symbol_id = hash_str("answer");
        assert_eq!(context.symbol_ptrs.get(&symbol_id), Some(&pointer));

        let bytes = context.compile().unwrap();
        let parsed = ParsedProgram::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.symbols.get(&symbol_id), Some(&register));
        assert!(parsed.mutable_symbols.contains(&symbol_id));
        assert_eq!(
            parsed.dictionary.get(&symbol_id).map(String::as_str),
            Some("answer")
        );
    }

    #[test]
    fn constants_keep_their_alignment() {
        let mut context = CompileCtx::new();
        context.compile_const(&[1], ValueKind::Index).unwrap();
        context.compile_const(&[2; 8], ValueKind::Index).unwrap();

        assert_eq!(context.const_entries[0].offset, 0);
        assert_eq!(context.const_entries[0].align, 8);
        assert_eq!(context.const_entries[1].offset, 8);
        assert_eq!(context.const_entries[1].align, 8);

        let parsed = ParsedProgram::from_bytes(&context.compile().unwrap()).unwrap();
        assert_eq!(parsed.const_entries[1].offset, 8);
    }

    #[test]
    fn instruction_emission_round_trips_unchanged() {
        let mut context = CompileCtx::new();
        context.emit_nullop(10, 0);
        context.emit_unop(11, 1, 0);
        context.emit_binop(12, 2, 0, 1);
        context.emit_ternop(13, 3, 0, 1, 2);
        context.emit_quadop(14, 4, 0, 1, 2, 3);
        context.emit_ret(5);
        context.emit_varop(15, 5, vec![0, 1, 2, 3, 4]);

        let parsed = ParsedProgram::from_bytes(&context.compile().unwrap()).unwrap();
        assert_eq!(
            parsed.instrs,
            vec![
                DecodedInstr::NullOp { fxn_id: 10, dst: 0 },
                DecodedInstr::UnOp {
                    fxn_id: 11,
                    dst: 1,
                    src: 0
                },
                DecodedInstr::BinOp {
                    fxn_id: 12,
                    dst: 2,
                    lhs: 0,
                    rhs: 1
                },
                DecodedInstr::TernOp {
                    fxn_id: 13,
                    dst: 3,
                    a: 0,
                    b: 1,
                    c: 2
                },
                DecodedInstr::QuadOp {
                    fxn_id: 14,
                    dst: 4,
                    a: 0,
                    b: 1,
                    c: 2,
                    d: 3
                },
                DecodedInstr::Ret { src: 5 },
                DecodedInstr::VarArg {
                    fxn_id: 15,
                    dst: 5,
                    args: vec![0, 1, 2, 3, 4]
                },
            ],
        );
    }

    #[test]
    fn emitted_sections_retain_version_one_layout_and_checksum() {
        let mut context = CompileCtx::new();
        context.require(FeatureFlag::Builtin(FeatureKind::Index));
        context.compile_const(&[1; 8], ValueKind::Index).unwrap();
        let (register, _) = context.register_for_ptr_with_initialization_status(1);
        context.define_symbol(1, register, "index", false);
        context.emit_const_load(register, 0);

        let bytes = context.compile().unwrap();
        let parsed = ParsedProgram::from_bytes(&bytes).unwrap();
        let header = &parsed.header;
        assert_eq!(header.version, 1);
        assert_eq!(header.feature_off, ByteCodeHeader::HEADER_SIZE as u64);
        assert_eq!(
            header.types_off,
            header.feature_off + 4 + (header.feature_count as u64 * 8),
        );
        assert_eq!(
            header.const_tbl_off,
            header.types_off + parsed.types.byte_len()
        );
        assert_eq!(
            header.const_blob_off,
            header.const_tbl_off + header.const_tbl_len
        );
        assert_eq!(
            header.symbols_off,
            header.const_blob_off + header.const_blob_len
        );
        assert_eq!(header.instr_off, header.symbols_off + header.symbols_len);
        assert_eq!(header.dict_off, header.instr_off + header.instr_len);
        assert_eq!(bytes.len() as u64, header.dict_off + header.dict_len + 4);

        let mut corrupted = bytes;
        corrupted[ByteCodeHeader::HEADER_SIZE] ^= 1;
        assert!(ParsedProgram::from_bytes(&corrupted).is_err());
    }

    #[test]
    fn requirements_are_deduplicated() {
        let mut context = CompileCtx::new();
        let requirement = FeatureFlag::Builtin(FeatureKind::Add);
        context.require(requirement.clone());
        context.require(requirement.clone());
        assert_eq!(context.requirements().len(), 1);
        assert!(context.requirements().contains(&requirement));
    }

    #[test]
    fn malformed_writer_positions_return_structured_errors() {
        let position_error = validate_buffer_position(4, 5).unwrap_err();
        let position = position_error
            .kind_as::<BufferPositionMismatchError>()
            .unwrap();
        assert_eq!(position.expected, 5);
        assert_eq!(position.got, 4);

        let length_error = validate_final_buffer_length(8, 9).unwrap_err();
        let length = length_error
            .kind_as::<FinalBufferLengthMismatchError>()
            .unwrap();
        assert_eq!(length.expected, 9);
        assert_eq!(length.got, 8);
    }
}
