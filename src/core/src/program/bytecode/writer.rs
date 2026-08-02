#[cfg(feature = "no_std")]
use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::String,
    vec::Vec,
};
#[cfg(not(feature = "no_std"))]
use std::collections::{BTreeMap, BTreeSet};

use crate::{MResult, hash_str};

use super::*;

#[derive(Clone, Debug)]
pub struct BytecodeProgram {
    pub register_count: u32,
    pub constants: Vec<EncodedConstant>,
    pub symbols: BTreeMap<u64, u32>,
    pub mutable_symbols: BTreeSet<u64>,
    pub instructions: Vec<BytecodeInstruction>,
    pub dictionary: BTreeMap<u64, String>,
    pub requirements: Vec<ApplicationRequirement>,
}

pub fn write_bytecode(program: &BytecodeProgram) -> MResult<Vec<u8>> {
    validate_writer_program(program)?;
    let (types, type_ids) = finalize_runtime_types(
        program
            .constants
            .iter()
            .map(|constant| &constant.runtime_type),
    )?;
    let types_bytes = encode_types(&types, &type_ids)?;
    let (constant_table_bytes, constant_blob_bytes) =
        encode_constants(&program.constants, &type_ids)?;
    let symbol_bytes = encode_symbols(program);
    let instruction_bytes = encode_instructions(&program.instructions)?;
    let dictionary_bytes = encode_dictionary(&program.dictionary)?;
    let requirement_bytes = encode_requirements(&program.requirements)?;
    let contents = [
        types_bytes,
        constant_table_bytes,
        constant_blob_bytes,
        symbol_bytes,
        instruction_bytes,
        dictionary_bytes,
        requirement_bytes,
    ];
    let counts = [
        count_u32(types.len(), "runtime types")?,
        count_u32(program.constants.len(), "constants")?,
        0,
        count_u32(program.symbols.len(), "symbols")?,
        count_u32(program.instructions.len(), "instructions")?,
        count_u32(program.dictionary.len(), "dictionary entries")?,
        count_u32(program.requirements.len(), "application requirements")?,
    ];

    let mut sections = Vec::with_capacity(BYTECODE_SECTION_COUNT);
    let mut offset = BYTECODE_CONTENT_OFFSET;
    for ((kind, bytes), item_count) in BytecodeSectionKind::ALL
        .into_iter()
        .zip(&contents)
        .zip(counts)
    {
        offset = align_up(offset, 8)?;
        let length = u64::try_from(bytes.len())
            .map_err(|_| invalid::<()>("section length exceeds u64").unwrap_err())?;
        sections.push(BytecodeSectionEntry {
            kind,
            flags: 0,
            item_count,
            offset,
            length,
            reserved: 0,
        });
        offset = offset
            .checked_add(length)
            .ok_or_else(|| invalid::<()>("bytecode file length overflow").unwrap_err())?;
    }
    let checksum_offset = offset;
    let file_len = checksum_offset
        .checked_add(4)
        .ok_or_else(|| invalid::<()>("bytecode file length overflow").unwrap_err())?;
    let (mech_major, mech_minor, mech_patch) = MECH_LANGUAGE_RUNTIME_ABI_VERSION;
    let header = BytecodeHeader {
        magic: BYTECODE_MAGIC,
        version: BYTECODE_VERSION,
        header_size: BYTECODE_HEADER_SIZE,
        mech_major,
        mech_minor,
        mech_patch,
        flags: 0,
        register_count: program.register_count,
        instruction_count: program
            .instructions
            .len()
            .try_into()
            .map_err(|_| invalid::<()>("too many bytecode instructions").unwrap_err())?,
        section_count: BYTECODE_SECTION_COUNT as u16,
        reserved0: 0,
        section_table_offset: BYTECODE_SECTION_TABLE_OFFSET,
        file_len,
        checksum_offset,
        reserved: [0; 12],
    };
    let capacity = usize::try_from(file_len)
        .map_err(|_| invalid::<()>("bytecode file exceeds address space").unwrap_err())?;
    let mut output = Vec::with_capacity(capacity);
    encode_header(&header, &mut output);
    for section in &sections {
        encode_section(section, &mut output);
    }
    for (section, bytes) in sections.iter().zip(contents) {
        let expected = usize::try_from(section.offset)
            .map_err(|_| invalid::<()>("section offset exceeds address space").unwrap_err())?;
        if output.len() > expected {
            return invalid("bytecode sections overlap");
        }
        output.resize(expected, 0);
        output.extend_from_slice(&bytes);
    }
    if u64::try_from(output.len())
        .map_err(|_| invalid::<()>("bytecode output length exceeds u64").unwrap_err())?
        != checksum_offset
    {
        return invalid("bytecode writer length mismatch");
    }
    let checksum = crc32fast::hash(&output);
    write_u32(&mut output, checksum);
    ParsedProgram::from_bytes(&output)?;
    Ok(output)
}

fn count_u32(value: usize, what: &str) -> MResult<u32> {
    value
        .try_into()
        .map_err(|_| invalid::<()>(format!("too many {what}")).unwrap_err())
}

fn validate_writer_program(program: &BytecodeProgram) -> MResult<()> {
    if program.instructions.len() > u32::MAX as usize {
        return invalid("too many bytecode instructions");
    }
    for (id, register) in &program.symbols {
        if *register >= program.register_count {
            return invalid("symbol register is out of range");
        }
        let name = program
            .dictionary
            .get(id)
            .ok_or_else(|| invalid::<()>("symbol is missing its dictionary name").unwrap_err())?;
        if name.is_empty() || hash_str(name) != *id {
            return invalid("symbol dictionary hash mismatch");
        }
    }
    for (id, name) in &program.dictionary {
        if name.is_empty() || hash_str(name) != *id {
            return invalid("dictionary hash mismatch");
        }
    }
    if program
        .mutable_symbols
        .iter()
        .any(|id| !program.symbols.contains_key(id))
    {
        return invalid("mutable symbol ID is missing from the symbol table");
    }
    if program.requirements.windows(2).any(|pair| {
        compare_application_requirements(&pair[0], &pair[1]) != core::cmp::Ordering::Less
    }) {
        return invalid("application requirements are not strictly sorted and deduplicated");
    }
    for requirement in &program.requirements {
        validate_application_requirement(requirement)?;
    }
    Ok(())
}

fn encode_header(header: &BytecodeHeader, out: &mut Vec<u8>) {
    out.extend_from_slice(&header.magic);
    write_u16(out, header.version);
    write_u16(out, header.header_size);
    write_u16(out, header.mech_major);
    write_u16(out, header.mech_minor);
    write_u16(out, header.mech_patch);
    write_u16(out, header.flags);
    write_u32(out, header.register_count);
    write_u32(out, header.instruction_count);
    write_u16(out, header.section_count);
    write_u16(out, header.reserved0);
    write_u64(out, header.section_table_offset);
    write_u64(out, header.file_len);
    write_u64(out, header.checksum_offset);
    out.extend_from_slice(&header.reserved);
}

fn encode_section(section: &BytecodeSectionEntry, out: &mut Vec<u8>) {
    write_u16(out, section.kind as u16);
    write_u16(out, section.flags);
    write_u32(out, section.item_count);
    write_u64(out, section.offset);
    write_u64(out, section.length);
    write_u64(out, section.reserved);
}

fn encode_types(types: &[RuntimeType], ids: &BTreeMap<RuntimeType, u32>) -> MResult<Vec<u8>> {
    let mut out = Vec::new();
    for ty in types {
        let payload = encode_type_payload(ty, ids)?;
        write_u16(&mut out, ty.tag() as u16);
        write_u16(&mut out, 0);
        write_u32(
            &mut out,
            payload
                .len()
                .try_into()
                .map_err(|_| invalid::<()>("runtime type payload exceeds u32").unwrap_err())?,
        );
        out.extend_from_slice(&payload);
    }
    Ok(out)
}

fn encode_constants(
    constants: &[EncodedConstant],
    ids: &BTreeMap<RuntimeType, u32>,
) -> MResult<(Vec<u8>, Vec<u8>)> {
    let table_capacity = constants
        .len()
        .checked_mul(24)
        .ok_or_else(|| invalid::<()>("constant table allocation overflow").unwrap_err())?;
    let mut table = Vec::with_capacity(table_capacity);
    let mut blob = Vec::new();
    for constant in constants {
        if !matches!(constant.alignment, 1 | 2 | 4 | 8 | 16) {
            return invalid("invalid encoded constant alignment");
        }
        let blob_len = u64::try_from(blob.len())
            .map_err(|_| invalid::<()>("constant blob length exceeds u64").unwrap_err())?;
        let offset = align_up(blob_len, u64::from(constant.alignment))?;
        let offset_usize = usize::try_from(offset)
            .map_err(|_| invalid::<()>("constant offset exceeds address space").unwrap_err())?;
        blob.resize(offset_usize, 0);
        blob.extend_from_slice(&constant.bytes);
        let type_id = ids
            .get(&constant.runtime_type)
            .copied()
            .ok_or_else(|| invalid::<()>("constant runtime type was not finalized").unwrap_err())?;
        write_u32(&mut table, type_id);
        table.push(1);
        table.push(constant.alignment);
        write_u16(&mut table, 0);
        write_u64(&mut table, offset);
        write_u64(
            &mut table,
            u64::try_from(constant.bytes.len())
                .map_err(|_| invalid::<()>("constant payload length exceeds u64").unwrap_err())?,
        );
    }
    Ok((table, blob))
}

fn encode_symbols(program: &BytecodeProgram) -> Vec<u8> {
    let mut out = Vec::with_capacity(program.symbols.len().saturating_mul(16));
    for (id, register) in &program.symbols {
        write_u64(&mut out, *id);
        write_u32(&mut out, *register);
        write_u32(&mut out, u32::from(program.mutable_symbols.contains(id)));
    }
    out
}

fn encode_instructions(instructions: &[BytecodeInstruction]) -> MResult<Vec<u8>> {
    let mut out = Vec::new();
    for instruction in instructions {
        match instruction {
            BytecodeInstruction::ConstLoad { dst, constant } => {
                out.push(Opcode::ConstLoad as u8);
                write_u32(&mut out, *dst);
                write_u32(&mut out, *constant);
            }
            BytecodeInstruction::RuntimeNullary { function, dst } => {
                out.push(Opcode::RuntimeNullary as u8);
                write_u64(&mut out, *function);
                write_u32(&mut out, *dst);
            }
            BytecodeInstruction::RuntimeUnary { function, dst, src } => {
                out.push(Opcode::RuntimeUnary as u8);
                write_u64(&mut out, *function);
                write_u32(&mut out, *dst);
                write_u32(&mut out, *src);
            }
            BytecodeInstruction::RuntimeBinary {
                function,
                dst,
                lhs,
                rhs,
            } => {
                out.push(Opcode::RuntimeBinary as u8);
                write_u64(&mut out, *function);
                write_u32(&mut out, *dst);
                write_u32(&mut out, *lhs);
                write_u32(&mut out, *rhs);
            }
            BytecodeInstruction::RuntimeTernary {
                function,
                dst,
                a,
                b,
                c,
            } => {
                out.push(Opcode::RuntimeTernary as u8);
                write_u64(&mut out, *function);
                for value in [dst, a, b, c] {
                    write_u32(&mut out, *value);
                }
            }
            BytecodeInstruction::RuntimeQuaternary {
                function,
                dst,
                a,
                b,
                c,
                d,
            } => {
                out.push(Opcode::RuntimeQuaternary as u8);
                write_u64(&mut out, *function);
                for value in [dst, a, b, c, d] {
                    write_u32(&mut out, *value);
                }
            }
            BytecodeInstruction::RuntimeVariadic {
                function,
                dst,
                arguments,
            } => {
                out.push(Opcode::RuntimeVariadic as u8);
                write_u64(&mut out, *function);
                write_u32(&mut out, *dst);
                write_u32(
                    &mut out,
                    arguments
                        .len()
                        .try_into()
                        .map_err(|_| invalid::<()>("too many variadic arguments").unwrap_err())?,
                );
                for argument in arguments {
                    write_u32(&mut out, *argument);
                }
            }
            BytecodeInstruction::HostCall {
                requirement,
                dst,
                arguments,
            } => {
                out.push(Opcode::HostCall as u8);
                write_u32(&mut out, *requirement);
                write_u32(&mut out, *dst);
                write_u32(
                    &mut out,
                    arguments
                        .len()
                        .try_into()
                        .map_err(|_| invalid::<()>("too many host call arguments").unwrap_err())?,
                );
                for argument in arguments {
                    write_u32(&mut out, *argument);
                }
            }
            BytecodeInstruction::ResourceRead { requirement, dst } => {
                out.push(Opcode::ResourceRead as u8);
                write_u32(&mut out, *requirement);
                write_u32(&mut out, *dst);
            }
            BytecodeInstruction::ResourceWrite {
                requirement,
                dst,
                src,
            } => {
                out.push(Opcode::ResourceWrite as u8);
                write_u32(&mut out, *requirement);
                write_u32(&mut out, *dst);
                write_u32(&mut out, *src);
            }
            BytecodeInstruction::ResourceSend {
                requirement,
                dst,
                src,
            } => {
                out.push(Opcode::ResourceSend as u8);
                write_u32(&mut out, *requirement);
                write_u32(&mut out, *dst);
                write_u32(&mut out, *src);
            }
            BytecodeInstruction::Return { src } => {
                out.push(Opcode::Return as u8);
                write_u32(&mut out, *src);
            }
        }
    }
    Ok(out)
}

fn encode_dictionary(dictionary: &BTreeMap<u64, String>) -> MResult<Vec<u8>> {
    let mut out = Vec::new();
    for (id, name) in dictionary {
        write_u64(&mut out, *id);
        write_string(&mut out, name)?;
    }
    Ok(out)
}

fn encode_requirements(requirements: &[ApplicationRequirement]) -> MResult<Vec<u8>> {
    let mut out = Vec::new();
    for requirement in requirements {
        let (kind, intent, delivery, operation, context, primary, secondary) = match requirement {
            ApplicationRequirement::HostFunction(request) => {
                (1, 0, 0, "", "", request.name.as_str(), "")
            }
            ApplicationRequirement::Resource(request) => (
                2,
                request.intent as u8,
                request.delivery as u8,
                request.operation.as_str(),
                request.context_name.as_str(),
                request.base_uri.as_str(),
                request.path.as_str(),
            ),
        };
        if primary.is_empty() {
            return invalid("application requirement primary field is empty");
        }
        out.extend_from_slice(&[kind, intent, delivery, 0]);
        write_u16(
            &mut out,
            operation
                .len()
                .try_into()
                .map_err(|_| invalid::<()>("requirement operation exceeds u16").unwrap_err())?,
        );
        write_u16(
            &mut out,
            context
                .len()
                .try_into()
                .map_err(|_| invalid::<()>("requirement context exceeds u16").unwrap_err())?,
        );
        write_u32(
            &mut out,
            primary
                .len()
                .try_into()
                .map_err(|_| invalid::<()>("requirement primary exceeds u32").unwrap_err())?,
        );
        write_u32(
            &mut out,
            secondary
                .len()
                .try_into()
                .map_err(|_| invalid::<()>("requirement secondary exceeds u32").unwrap_err())?,
        );
        out.extend_from_slice(operation.as_bytes());
        out.extend_from_slice(context.as_bytes());
        out.extend_from_slice(primary.as_bytes());
        out.extend_from_slice(secondary.as_bytes());
    }
    Ok(out)
}
