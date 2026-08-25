#[cfg(feature = "no_std")]
use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::String,
    vec::Vec,
};
#[cfg(not(feature = "no_std"))]
use std::collections::{BTreeMap, BTreeSet};
#[cfg(not(feature = "no_std"))]
use std::fs;
#[cfg(not(feature = "no_std"))]
use std::path::Path;

use crate::{LegacyValue, MResult, MechError, hash_str};

use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedProgram {
    pub header: BytecodeHeader,
    pub sections: Vec<BytecodeSectionEntry>,
    pub types: Vec<RuntimeType>,
    pub constants: Vec<ConstantEntry>,
    pub constant_blob: Vec<u8>,
    pub symbols: BTreeMap<u64, u32>,
    pub mutable_symbols: BTreeSet<u64>,
    pub instructions: Vec<BytecodeInstruction>,
    pub dictionary: BTreeMap<u64, String>,
    pub requirements: Vec<ApplicationRequirement>,
    pub artifact: BytecodeArtifactSections,
}

impl ParsedProgram {
    pub fn from_bytes(bytes: &[u8]) -> MResult<Self> {
        Self::from_bytes_with_limits(bytes, BytecodeReadLimits::default())
    }

    pub fn from_bytes_with_limits(bytes: &[u8], limits: BytecodeReadLimits) -> MResult<Self> {
        parse_program(bytes, &limits)
    }

    pub fn decode_constants(&self) -> MResult<Vec<LegacyValue>> {
        decode_constants(&self.types, &self.constants, &self.constant_blob)
    }

    /// Returns every runtime type required to decode this program, including
    /// types stored inline by enum payloads rather than in the type table.
    pub fn referenced_runtime_types(&self) -> MResult<Vec<RuntimeType>> {
        constants::referenced_runtime_types(&self.types, &self.constants, &self.constant_blob)
    }

    /// Largest canonical `Index` payload, read independently of this process's
    /// pointer width for native cross-target validation.
    pub fn maximum_index_constant(&self) -> MResult<Option<u64>> {
        constants::maximum_index_constant(&self.types, &self.constants, &self.constant_blob)
    }
}

#[cfg(not(feature = "no_std"))]
pub fn load_program_from_file(path: impl AsRef<Path>) -> MResult<ParsedProgram> {
    ParsedProgram::from_bytes(&fs::read(path)?)
}

pub fn load_program_from_bytes(bytes: &[u8]) -> MResult<ParsedProgram> {
    ParsedProgram::from_bytes(bytes)
}

fn parse_program(bytes: &[u8], limits: &BytecodeReadLimits) -> MResult<ParsedProgram> {
    if bytes.len() > limits.max_file_bytes {
        return invalid("bytecode file exceeds read limit");
    }
    let minimum_file_bytes = checked_usize(BYTECODE_CONTENT_OFFSET, "bytecode content offset")?
        .checked_add(4)
        .ok_or_else(|| invalid::<()>("minimum bytecode file size overflow").unwrap_err())?;
    if bytes.len() < minimum_file_bytes {
        return invalid("bytecode file is shorter than header, section table, and checksum");
    }
    let header = parse_header(bytes)?;
    validate_header(&header, bytes.len(), limits)?;
    validate_checksum(bytes, header.checksum_offset)?;
    let sections = parse_sections(bytes, &header)?;
    validate_sections(bytes, &sections, header.checksum_offset)?;
    let section = |kind| sections.iter().find(|entry| entry.kind == kind).unwrap();

    let types_section = section(BytecodeSectionKind::Types);
    if types_section.item_count > limits.max_types {
        return invalid("runtime type count exceeds read limit");
    }
    let (raw_types, types) = parse_types(
        section_bytes(bytes, types_section)?,
        types_section.item_count,
    )?;
    let (canonical_types, _) = finalize_runtime_types(types.iter())?;
    if canonical_types != types {
        return invalid("runtime type IDs are not in canonical deterministic order");
    }

    let constants_section = section(BytecodeSectionKind::ConstantTable);
    if constants_section.item_count > limits.max_constants {
        return invalid("constant count exceeds read limit");
    }
    let constants = parse_constants(
        section_bytes(bytes, constants_section)?,
        constants_section.item_count,
    )?;
    let constant_blob_section = section(BytecodeSectionKind::ConstantBlob);
    if constant_blob_section.item_count != 0 {
        return invalid("ConstantBlob item count must be zero");
    }
    let constant_blob_bytes = section_bytes(bytes, constant_blob_section)?;
    let mut constant_blob = Vec::new();
    constant_blob
        .try_reserve_exact(constant_blob_bytes.len())
        .map_err(|_| invalid::<()>("unable to allocate ConstantBlob").unwrap_err())?;
    constant_blob.extend_from_slice(constant_blob_bytes);
    validate_constant_entries(&types, &constants, &constant_blob)?;
    validate_type_reachability(&raw_types, &constants)?;

    let symbols_section = section(BytecodeSectionKind::Symbols);
    if symbols_section.item_count > limits.max_symbols {
        return invalid("symbol count exceeds read limit");
    }
    let (symbols, mutable_symbols) = parse_symbols(
        section_bytes(bytes, symbols_section)?,
        symbols_section.item_count,
        header.register_count,
    )?;

    let instructions_section = section(BytecodeSectionKind::Instructions);
    if instructions_section.item_count != header.instruction_count {
        return invalid("instruction section count disagrees with header");
    }
    let instructions = parse_instructions(
        section_bytes(bytes, instructions_section)?,
        header.instruction_count,
        limits.max_variadic_arguments,
    )?;

    let dictionary_section = section(BytecodeSectionKind::Dictionary);
    if dictionary_section.item_count > limits.max_dictionary_entries {
        return invalid("dictionary entry count exceeds read limit");
    }
    let dictionary_bytes = checked_usize(dictionary_section.length, "dictionary section length")?;
    if dictionary_bytes > limits.max_dictionary_bytes {
        return invalid("dictionary bytes exceed read limit");
    }
    let dictionary = parse_dictionary(
        section_bytes(bytes, dictionary_section)?,
        dictionary_section.item_count,
    )?;

    let requirements_section = section(BytecodeSectionKind::ApplicationRequirements);
    if requirements_section.item_count > limits.max_requirements {
        return invalid("application requirement count exceeds read limit");
    }
    let requirements = parse_requirements(
        section_bytes(bytes, requirements_section)?,
        requirements_section.item_count,
    )?;
    if requirements.windows(2).any(|pair| {
        compare_application_requirements(&pair[0], &pair[1]) != core::cmp::Ordering::Less
    }) {
        return invalid("application requirements are not strictly sorted and deduplicated");
    }

    let artifact_kinds = [
        BytecodeSectionKind::ArtifactSchemas,
        BytecodeSectionKind::ArtifactConstants,
        BytecodeSectionKind::ArtifactInputs,
        BytecodeSectionKind::ArtifactSlots,
        BytecodeSectionKind::ArtifactProducers,
        BytecodeSectionKind::ArtifactNodes,
        BytecodeSectionKind::ArtifactBindings,
        BytecodeSectionKind::ArtifactOutputs,
        BytecodeSectionKind::ArtifactIntegrityConstraints,
        BytecodeSectionKind::ArtifactOperations,
        BytecodeSectionKind::ArtifactOperationContracts,
    ];
    let mut artifact_bytes = Vec::with_capacity(artifact_kinds.len() + 1);
    let mut total_artifact_bytes = 0_usize;
    for kind in artifact_kinds {
        let entry = section(kind);
        let bytes = section_bytes(bytes, entry)?;
        if bytes.len() > limits.max_artifact_section_bytes {
            return invalid("ProgramArtifact section exceeds read limit");
        }
        total_artifact_bytes = total_artifact_bytes
            .checked_add(bytes.len())
            .ok_or_else(|| invalid::<()>("ProgramArtifact byte count overflow").unwrap_err())?;
        if total_artifact_bytes > limits.max_artifact_bytes {
            return invalid("ProgramArtifact sections exceed total read limit");
        }
        if entry.item_count != u32::from(!bytes.is_empty()) {
            return invalid("ProgramArtifact section item count must describe presence");
        }
        artifact_bytes.push(bytes.to_vec());
    }
    let compute_region_bytes = if let Some(entry) = sections
        .iter()
        .find(|entry| entry.kind == BytecodeSectionKind::ArtifactComputeRegions)
    {
        let bytes = section_bytes(bytes, entry)?;
        if bytes.len() > limits.max_artifact_section_bytes {
            return invalid("ProgramArtifact section exceeds read limit");
        }
        total_artifact_bytes = total_artifact_bytes
            .checked_add(bytes.len())
            .ok_or_else(|| invalid::<()>("ProgramArtifact byte count overflow").unwrap_err())?;
        if total_artifact_bytes > limits.max_artifact_bytes {
            return invalid("ProgramArtifact sections exceed total read limit");
        }
        if entry.item_count != u32::from(!bytes.is_empty()) {
            return invalid("ProgramArtifact section item count must describe presence");
        }
        bytes.to_vec()
    } else {
        Vec::new()
    };
    let mut artifact_bytes = artifact_bytes.into_iter();
    let artifact = BytecodeArtifactSections {
        schemas: artifact_bytes.next().unwrap(),
        constants: artifact_bytes.next().unwrap(),
        inputs: artifact_bytes.next().unwrap(),
        slots: artifact_bytes.next().unwrap(),
        producers: artifact_bytes.next().unwrap(),
        nodes: artifact_bytes.next().unwrap(),
        bindings: artifact_bytes.next().unwrap(),
        outputs: artifact_bytes.next().unwrap(),
        integrity_constraints: artifact_bytes.next().unwrap(),
        operations: artifact_bytes.next().unwrap(),
        operation_contracts: artifact_bytes.next().unwrap(),
        compute_regions: compute_region_bytes,
    };
    let base_artifact_sections = &artifact.ordered()[..11];
    let base_artifact_is_empty = base_artifact_sections
        .iter()
        .all(|section| section.is_empty());
    if base_artifact_sections
        .iter()
        .any(|section| section.is_empty())
        && !base_artifact_is_empty
    {
        return invalid("ProgramArtifact bytecode sections must be all present or all absent");
    }
    if base_artifact_is_empty && !artifact.compute_regions.is_empty() {
        return invalid("compute-region metadata requires a ProgramArtifact");
    }

    for (id, _) in &symbols {
        let name = dictionary.get(id).ok_or_else(|| {
            invalid::<()>("symbol is missing its exact dictionary name").unwrap_err()
        })?;
        if hash_str(name) != *id {
            return invalid("symbol dictionary hash mismatch");
        }
    }
    if let Some(id) = dictionary.keys().find(|id| !symbols.contains_key(id)) {
        return invalid(format!(
            "dictionary entry {id} is not referenced by a symbol"
        ));
    }
    let initialized = validate_instructions(
        &instructions,
        &header,
        constants.len(),
        requirements.len(),
        !artifact.is_empty(),
    )?;
    validate_composite_packs(
        &instructions,
        header.register_count as usize,
        &types,
        &constants,
        &constant_blob,
    )?;
    validate_constant_and_requirement_reachability(
        &instructions,
        constants.len(),
        requirements.len(),
        !artifact.is_empty(),
    )?;
    for register in symbols.values().copied() {
        if !initialized[register as usize] {
            return invalid(format!(
                "symbol register {register} is uninitialized after instruction validation"
            ));
        }
    }
    let canonical_register_count = initialized
        .iter()
        .rposition(|initialized| *initialized)
        .map(|register| register + 1)
        .unwrap_or(0);
    if header.register_count as usize != canonical_register_count {
        return invalid(format!(
            "register count {} does not match highest referenced register count {canonical_register_count}",
            header.register_count,
        ));
    }

    Ok(ParsedProgram {
        header,
        sections,
        types,
        constants,
        constant_blob,
        symbols,
        mutable_symbols,
        instructions,
        dictionary,
        requirements,
        artifact,
    })
}

fn validate_composite_packs(
    instructions: &[BytecodeInstruction],
    register_count: usize,
    types: &[RuntimeType],
    constants: &[ConstantEntry],
    blob: &[u8],
) -> MResult<()> {
    if !instructions
        .iter()
        .any(|instruction| matches!(instruction, BytecodeInstruction::CompositePack { .. }))
    {
        return Ok(());
    }
    let values = decode_constants(types, constants, blob)?;
    let mut registers = vec![None::<LegacyValue>; register_count];
    let mut dynamic = vec![false; register_count];
    for instruction in instructions {
        match instruction {
            BytecodeInstruction::ConstLoad { dst, constant } => {
                registers[*dst as usize] = Some(values[*constant as usize].clone());
            }
            BytecodeInstruction::CompositePack {
                dst,
                template,
                children,
            } => {
                let template = &values[*template as usize];
                let static_children = children
                    .iter()
                    .map(|child| registers[*child as usize].clone())
                    .collect::<Option<Vec<_>>>();
                if let Some(children) = static_children {
                    registers[*dst as usize] =
                        Some(crate::rebuild_bytecode_composite(template, children)?);
                } else {
                    for child in children {
                        if registers[*child as usize].is_none() && !dynamic[*child as usize] {
                            return invalid(format!(
                                "CompositePack child register {child} has no producer"
                            ));
                        }
                    }
                    let template_children = crate::bytecode_composite_children(template)
                        .ok_or_else(|| {
                            invalid::<()>("CompositePack template is not a composite value")
                                .unwrap_err()
                        })?;
                    if template_children.len() != children.len() {
                        return invalid(format!(
                            "CompositePack template expects {} children, found {}",
                            template_children.len(),
                            children.len(),
                        ));
                    }
                    dynamic[*dst as usize] = true;
                }
            }
            BytecodeInstruction::ResourceRead { dst, .. } => dynamic[*dst as usize] = true,
            _ => {}
        }
    }
    Ok(())
}

fn parse_header(bytes: &[u8]) -> MResult<BytecodeHeader> {
    let mut r = ByteReader::new(
        bytes
            .get(..usize::from(BYTECODE_HEADER_SIZE))
            .ok_or_else(|| invalid::<()>("truncated bytecode header").unwrap_err())?,
    );
    let mut magic = [0; 4];
    magic.copy_from_slice(r.read_exact(4, "bytecode magic")?);
    let version = r.read_u16("bytecode version")?;
    let header_size = r.read_u16("header size")?;
    let mech_major = r.read_u16("Mech major version")?;
    let mech_minor = r.read_u16("Mech minor version")?;
    let mech_patch = r.read_u16("Mech patch version")?;
    let flags = r.read_u16("header flags")?;
    let register_count = r.read_u32("register count")?;
    let instruction_count = r.read_u32("instruction count")?;
    let section_count = r.read_u16("section count")?;
    let reserved0 = r.read_u16("reserved0")?;
    let section_table_offset = r.read_u64("section table offset")?;
    let file_len = r.read_u64("file length")?;
    let checksum_offset = r.read_u64("checksum offset")?;
    let mut reserved = [0; 12];
    reserved.copy_from_slice(r.read_exact(12, "reserved header bytes")?);
    Ok(BytecodeHeader {
        magic,
        version,
        header_size,
        mech_major,
        mech_minor,
        mech_patch,
        flags,
        register_count,
        instruction_count,
        section_count,
        reserved0,
        section_table_offset,
        file_len,
        checksum_offset,
        reserved,
    })
}

fn validate_header(
    header: &BytecodeHeader,
    actual_len: usize,
    limits: &BytecodeReadLimits,
) -> MResult<()> {
    if header.magic != BYTECODE_MAGIC {
        return invalid("wrong bytecode magic");
    }
    if header.version != BYTECODE_VERSION {
        return invalid("wrong bytecode version");
    }
    if header.header_size != BYTECODE_HEADER_SIZE {
        return invalid("wrong bytecode header size");
    }
    if (header.mech_major, header.mech_minor, header.mech_patch)
        != MECH_LANGUAGE_RUNTIME_ABI_VERSION
    {
        return invalid("wrong Mech language/runtime ABI version");
    }
    if header.flags != 0 || header.reserved0 != 0 || header.reserved != [0; 12] {
        return invalid("reserved header fields must be zero");
    }
    if header.register_count > limits.max_registers
        || header.instruction_count > limits.max_instructions
    {
        return invalid("bytecode header count exceeds read limit");
    }
    if !matches!(
        usize::from(header.section_count),
        BYTECODE_SECTION_COUNT | BYTECODE_SECTION_COUNT_WITH_COMPUTE_REGIONS
    ) || header.section_table_offset != BYTECODE_SECTION_TABLE_OFFSET
    {
        return invalid("bytecode must contain a supported section table at offset 64");
    }
    let actual_len = u64::try_from(actual_len)
        .map_err(|_| invalid::<()>("actual bytecode length exceeds u64").unwrap_err())?;
    if header.file_len != actual_len {
        return invalid("header file length disagrees with actual input length");
    }
    if header.checksum_offset
        != header
            .file_len
            .checked_sub(4)
            .ok_or_else(|| invalid::<()>("checksum offset underflow").unwrap_err())?
    {
        return invalid("checksum offset must equal file length minus four");
    }
    Ok(())
}

fn validate_checksum(bytes: &[u8], checksum_offset: u64) -> MResult<()> {
    let offset = checked_usize(checksum_offset, "checksum offset")?;
    let checksum_end = offset
        .checked_add(4)
        .ok_or_else(|| invalid::<()>("checksum range overflow").unwrap_err())?;
    let expected = u32::from_le_bytes(
        bytes
            .get(offset..checksum_end)
            .ok_or_else(|| invalid::<()>("truncated checksum").unwrap_err())?
            .try_into()
            .unwrap(),
    );
    let actual = crc32fast::hash(&bytes[..offset]);
    if expected != actual {
        return invalid("CRC32 checksum mismatch");
    }
    Ok(())
}

fn parse_sections(bytes: &[u8], header: &BytecodeHeader) -> MResult<Vec<BytecodeSectionEntry>> {
    let start = checked_usize(header.section_table_offset, "section table offset")?;
    let section_count = usize::from(header.section_count);
    let end = start
        .checked_add(section_count * BYTECODE_SECTION_ENTRY_SIZE)
        .ok_or_else(|| invalid::<()>("section table overflow").unwrap_err())?;
    let mut r = ByteReader::new(
        bytes
            .get(start..end)
            .ok_or_else(|| invalid::<()>("truncated section table").unwrap_err())?,
    );
    let mut sections = Vec::with_capacity(section_count);
    for expected in BytecodeSectionKind::ALL_WITH_COMPUTE_REGIONS
        .into_iter()
        .take(section_count)
    {
        let raw_kind = r.read_u16("section kind")?;
        let kind = BytecodeSectionKind::from_u16(raw_kind)
            .ok_or_else(|| invalid::<()>("unknown bytecode section kind").unwrap_err())?;
        if kind != expected {
            return invalid("missing, duplicate, or out-of-order bytecode section");
        }
        sections.push(BytecodeSectionEntry {
            kind,
            flags: r.read_u16("section flags")?,
            item_count: r.read_u32("section item count")?,
            offset: r.read_u64("section offset")?,
            length: r.read_u64("section length")?,
            reserved: r.read_u64("section reserved")?,
        });
    }
    Ok(sections)
}

fn validate_sections(
    bytes: &[u8],
    sections: &[BytecodeSectionEntry],
    checksum_offset: u64,
) -> MResult<()> {
    let content_offset = match sections.len() {
        BYTECODE_SECTION_COUNT => BYTECODE_CONTENT_OFFSET,
        BYTECODE_SECTION_COUNT_WITH_COMPUTE_REGIONS => BYTECODE_CONTENT_OFFSET_WITH_COMPUTE_REGIONS,
        _ => return invalid("bytecode contains an unsupported section count"),
    };
    if sections.first().map(|section| section.offset) != Some(content_offset) {
        return invalid("first bytecode content section has the wrong offset");
    }
    let mut previous_end = content_offset;
    let checksum_end = checked_usize(checksum_offset, "checksum offset")?;
    for section in sections {
        if section.flags != 0 || section.reserved != 0 {
            return invalid("section flags and reserved fields must be zero");
        }
        let expected_offset = align_up(previous_end, 8)?;
        if section.offset != expected_offset {
            return invalid("section does not begin at its minimal aligned offset");
        }
        let end = section
            .offset
            .checked_add(section.length)
            .ok_or_else(|| invalid::<()>("section range overflow").unwrap_err())?;
        if end > checksum_offset {
            return invalid("section extends into checksum");
        }
        let padding_start = checked_usize(previous_end, "previous section end")?;
        let padding_end = checked_usize(section.offset, "section offset")?;
        let padding = bytes
            .get(padding_start..padding_end)
            .ok_or_else(|| invalid::<()>("section padding is out of bounds").unwrap_err())?;
        if padding.iter().any(|byte| *byte != 0) {
            return invalid("section padding must be zero");
        }
        previous_end = end;
    }
    if previous_end != checksum_offset {
        return invalid("checksum does not immediately follow the final section");
    }
    if checked_usize(previous_end, "final section end")? != checksum_end {
        return invalid("checksum offset exceeds address space");
    }
    Ok(())
}

fn section_bytes<'a>(bytes: &'a [u8], section: &BytecodeSectionEntry) -> MResult<&'a [u8]> {
    let start = usize::try_from(section.offset)
        .map_err(|_| invalid::<()>("section offset exceeds address space").unwrap_err())?;
    let length = usize::try_from(section.length)
        .map_err(|_| invalid::<()>("section length exceeds address space").unwrap_err())?;
    let end = start
        .checked_add(length)
        .ok_or_else(|| invalid::<()>("section slice overflow").unwrap_err())?;
    bytes
        .get(start..end)
        .ok_or_else(|| invalid::<()>("section is out of bounds").unwrap_err())
}

fn checked_item_count(count: u32, what: &str) -> MResult<usize> {
    checked_usize(u64::from(count), what)
}

fn validate_minimum_bytes(
    count: usize,
    minimum_item_bytes: usize,
    available_bytes: usize,
    what: &str,
) -> MResult<()> {
    let minimum_bytes = count
        .checked_mul(minimum_item_bytes)
        .ok_or_else(|| invalid::<()>(format!("{what} byte length overflow")).unwrap_err())?;
    if minimum_bytes > available_bytes {
        return invalid(format!("{what} exceeds section capacity"));
    }
    Ok(())
}

fn try_vec_with_capacity<T>(capacity: usize, what: &str) -> MResult<Vec<T>> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(capacity)
        .map_err(|_| invalid::<()>(format!("unable to allocate {what}")).unwrap_err())?;
    Ok(values)
}

fn parse_types(bytes: &[u8], count: u32) -> MResult<(Vec<RawRuntimeType>, Vec<RuntimeType>)> {
    let mut r = ByteReader::new(bytes);
    let count = usize::try_from(count)
        .map_err(|_| invalid::<()>("runtime type count exceeds address space").unwrap_err())?;
    let minimum_length = count
        .checked_mul(8)
        .ok_or_else(|| invalid::<()>("runtime type section length overflow").unwrap_err())?;
    if minimum_length > r.remaining() {
        return invalid("runtime type count exceeds section capacity");
    }
    let mut raw = Vec::new();
    raw.try_reserve_exact(count)
        .map_err(|_| invalid::<()>("unable to allocate runtime type table").unwrap_err())?;
    for _ in 0..count {
        let tag = RuntimeTypeTag::from_u16(r.read_u16("runtime type tag")?)
            .ok_or_else(|| invalid::<()>("unknown runtime type tag").unwrap_err())?;
        if r.read_u16("runtime type flags")? != 0 {
            return invalid("runtime type flags must be zero");
        }
        let length = usize::try_from(r.read_u32("runtime type payload length")?).map_err(|_| {
            invalid::<()>("runtime type payload length exceeds address space").unwrap_err()
        })?;
        let payload = r.read_exact(length, "runtime type payload")?;
        raw.push(decode_raw_type(tag, payload)?);
    }
    if !r.is_empty() {
        return invalid("type section has trailing bytes");
    }
    let resolved = resolve_raw_types(&raw)?;
    Ok((raw, resolved))
}

fn validate_type_reachability(
    raw_types: &[RawRuntimeType],
    constants: &[ConstantEntry],
) -> MResult<()> {
    fn visit(type_id: u32, raw_types: &[RawRuntimeType], reachable: &mut [bool]) -> MResult<()> {
        let index = checked_item_count(type_id, "reachable runtime type ID")?;
        let Some(raw) = raw_types.get(index) else {
            return invalid("constant references an out-of-range runtime type");
        };
        if reachable[index] {
            return Ok(());
        }
        reachable[index] = true;

        let mut child = |child_id| visit(child_id, raw_types, reachable);
        match raw {
            RawRuntimeType::Complete(_) => {}
            RawRuntimeType::Matrix { element, .. }
            | RawRuntimeType::Reference(element)
            | RawRuntimeType::Set { element, .. }
            | RawRuntimeType::Option(element) => child(*element)?,
            RawRuntimeType::Record(fields)
            | RawRuntimeType::Table {
                columns: fields, ..
            } => {
                for (_, child_id) in fields {
                    child(*child_id)?;
                }
            }
            RawRuntimeType::Map { key, value } => {
                child(*key)?;
                child(*value)?;
            }
            RawRuntimeType::Tuple(types) => {
                for child_id in types {
                    child(*child_id)?;
                }
            }
        }
        Ok(())
    }

    let mut reachable = vec![false; raw_types.len()];
    for constant in constants {
        visit(constant.type_id, raw_types, &mut reachable)?;
    }
    if let Some(type_id) = reachable.iter().position(|reachable| !reachable) {
        return Err(MechError::new(
            BytecodeUnreferencedType {
                type_id: u32::try_from(type_id).unwrap_or(u32::MAX),
            },
            None,
        )
        .with_compiler_loc());
    }
    Ok(())
}

fn validate_constant_and_requirement_reachability(
    instructions: &[BytecodeInstruction],
    constant_count: usize,
    requirement_count: usize,
    artifact_present: bool,
) -> MResult<()> {
    let mut constants = vec![false; constant_count];
    let mut requirements = vec![false; requirement_count];
    let mut next_constant_id = 0usize;
    for instruction in instructions {
        match instruction {
            BytecodeInstruction::ConstLoad { constant, .. }
            | BytecodeInstruction::CompositePack {
                template: constant, ..
            } => {
                let constant = *constant as usize;
                if !constants[constant] {
                    if constant != next_constant_id {
                        return invalid(format!(
                            "constant ID {constant} is noncanonical; expected first-reference ID {next_constant_id}"
                        ));
                    }
                    constants[constant] = true;
                    next_constant_id += 1;
                }
            }
            BytecodeInstruction::HostCall { requirement, .. }
            | BytecodeInstruction::ResourceRead { requirement, .. }
            | BytecodeInstruction::ResourceWrite { requirement, .. }
            | BytecodeInstruction::ResourceSend { requirement, .. } => {
                requirements[*requirement as usize] = true;
            }
            _ => {}
        }
    }
    if let Some(constant) = constants.iter().position(|referenced| !referenced) {
        return Err(MechError::new(
            BytecodeUnreferencedConstant {
                constant: u32::try_from(constant).unwrap_or(u32::MAX),
            },
            None,
        )
        .with_compiler_loc());
    }
    if !artifact_present
        && let Some(requirement) = requirements.iter().position(|referenced| !referenced)
    {
        return Err(MechError::new(
            BytecodeUnreferencedRequirement {
                requirement: u32::try_from(requirement).unwrap_or(u32::MAX),
            },
            None,
        )
        .with_compiler_loc());
    }
    Ok(())
}

fn parse_constants(bytes: &[u8], count: u32) -> MResult<Vec<ConstantEntry>> {
    let count = checked_item_count(count, "constant count")?;
    let expected = count
        .checked_mul(24)
        .ok_or_else(|| invalid::<()>("constant table length overflow").unwrap_err())?;
    if bytes.len() != expected {
        return invalid("constant table length disagrees with item count");
    }
    let mut r = ByteReader::new(bytes);
    let mut entries = try_vec_with_capacity(count, "constant table")?;
    for _ in 0..count {
        entries.push(ConstantEntry {
            type_id: r.read_u32("constant type ID")?,
            encoding: r.read_u8("constant encoding")?,
            alignment: r.read_u8("constant alignment")?,
            flags: r.read_u16("constant flags")?,
            offset: r.read_u64("constant offset")?,
            length: r.read_u64("constant length")?,
        });
    }
    Ok(entries)
}

fn validate_constant_entries(
    types: &[RuntimeType],
    entries: &[ConstantEntry],
    blob: &[u8],
) -> MResult<()> {
    let mut previous_end = 0usize;
    let mut canonical_entries = BTreeSet::new();
    for entry in entries {
        if entry.encoding != 1
            || entry.flags != 0
            || !matches!(entry.alignment, 1 | 2 | 4 | 8 | 16)
            || entry.offset % u64::from(entry.alignment) != 0
        {
            return invalid("invalid constant table entry");
        }
        let start = usize::try_from(entry.offset)
            .map_err(|_| invalid::<()>("constant offset exceeds address space").unwrap_err())?;
        let length = usize::try_from(entry.length)
            .map_err(|_| invalid::<()>("constant length exceeds address space").unwrap_err())?;
        let end = start
            .checked_add(length)
            .ok_or_else(|| invalid::<()>("constant range overflow").unwrap_err())?;
        let expected_start = usize::try_from(align_up(
            u64::try_from(previous_end)
                .map_err(|_| invalid::<()>("constant offset exceeds u64").unwrap_err())?,
            u64::from(entry.alignment),
        )?)
        .map_err(|_| invalid::<()>("constant offset exceeds address space").unwrap_err())?;
        if start != expected_start {
            return invalid("constant offset is not the minimal aligned offset");
        }
        if end > blob.len() {
            return invalid("constant entry exceeds ConstantBlob");
        }
        if blob[previous_end..start].iter().any(|byte| *byte != 0) {
            return invalid("constant alignment padding bytes must be zero");
        }
        let type_id = checked_item_count(entry.type_id, "constant type ID")?;
        let ty = types
            .get(type_id)
            .ok_or_else(|| invalid::<()>("constant type ID is out of range").unwrap_err())?;
        validate_constant_payload(ty, &blob[start..end])?;
        if !canonical_entries.insert((entry.type_id, &blob[start..end])) {
            return invalid(
                "constant table contains duplicate canonical runtime type and payload entries",
            );
        }
        previous_end = end;
    }
    if previous_end != blob.len() {
        return invalid("ConstantBlob contains noncanonical trailing bytes");
    }
    Ok(())
}

fn validate_constant_payload(ty: &RuntimeType, bytes: &[u8]) -> MResult<()> {
    super::constants::validate_constant_value_payload(ty, bytes)
}

fn parse_symbols(
    bytes: &[u8],
    count: u32,
    register_count: u32,
) -> MResult<(BTreeMap<u64, u32>, BTreeSet<u64>)> {
    let count = checked_item_count(count, "symbol count")?;
    let expected = count
        .checked_mul(16)
        .ok_or_else(|| invalid::<()>("symbol table length overflow").unwrap_err())?;
    if bytes.len() != expected {
        return invalid("symbol section length disagrees with item count");
    }
    let mut r = ByteReader::new(bytes);
    let mut symbols = BTreeMap::new();
    let mut mutable = BTreeSet::new();
    let mut previous = None;
    for _ in 0..count {
        let id = r.read_u64("symbol ID")?;
        let register = r.read_u32("symbol register")?;
        let flags = r.read_u32("symbol flags")?;
        if previous >= Some(id) || symbols.insert(id, register).is_some() {
            return invalid("symbols are duplicate or unsorted");
        }
        if register >= register_count {
            return invalid("symbol register is out of range");
        }
        if flags & !1 != 0 {
            return invalid("unknown symbol flag bits");
        }
        if flags & 1 != 0 {
            mutable.insert(id);
        }
        previous = Some(id);
    }
    Ok((symbols, mutable))
}

fn parse_register_arguments(
    reader: &mut ByteReader<'_>,
    encoded_count: u32,
    max_count: u32,
    what: &str,
) -> MResult<Vec<u32>> {
    if encoded_count > max_count {
        return invalid(format!("{what} exceeds read limit"));
    }
    let count = checked_item_count(encoded_count, what)?;
    let byte_count = count
        .checked_mul(4)
        .ok_or_else(|| invalid::<()>(format!("{what} byte length overflow")).unwrap_err())?;
    if byte_count > reader.remaining() {
        return invalid(format!("{what} exceeds remaining instruction bytes"));
    }
    let mut arguments = try_vec_with_capacity(count, what)?;
    for _ in 0..count {
        arguments.push(reader.read_u32("instruction argument")?);
    }
    Ok(arguments)
}

fn parse_instructions(
    bytes: &[u8],
    count: u32,
    max_variadic_arguments: u32,
) -> MResult<Vec<BytecodeInstruction>> {
    let mut r = ByteReader::new(bytes);
    let count = checked_item_count(count, "instruction count")?;
    validate_minimum_bytes(count, 5, r.remaining(), "instruction count")?;
    let mut instructions = try_vec_with_capacity(count, "instruction table")?;
    for _ in 0..count {
        let opcode = Opcode::from_u8(r.read_u8("instruction opcode")?)
            .ok_or_else(|| invalid::<()>("unknown bytecode opcode").unwrap_err())?;
        let instruction = match opcode {
            Opcode::ConstLoad => BytecodeInstruction::ConstLoad {
                dst: r.read_u32("ConstLoad destination")?,
                constant: r.read_u32("ConstLoad constant")?,
            },
            Opcode::CompositePack => {
                let dst = r.read_u32("CompositePack destination")?;
                let template = r.read_u32("CompositePack template")?;
                let child_count = r.read_u32("CompositePack child count")?;
                let children = parse_register_arguments(
                    &mut r,
                    child_count,
                    max_variadic_arguments,
                    "CompositePack child count",
                )?;
                BytecodeInstruction::CompositePack {
                    dst,
                    template,
                    children,
                }
            }
            Opcode::RuntimeNullary => BytecodeInstruction::RuntimeNullary {
                function: r.read_u64("runtime function ID")?,
                dst: r.read_u32("runtime destination")?,
            },
            Opcode::RuntimeUnary => BytecodeInstruction::RuntimeUnary {
                function: r.read_u64("runtime function ID")?,
                dst: r.read_u32("runtime destination")?,
                src: r.read_u32("runtime source")?,
            },
            Opcode::RuntimeBinary => BytecodeInstruction::RuntimeBinary {
                function: r.read_u64("runtime function ID")?,
                dst: r.read_u32("runtime destination")?,
                lhs: r.read_u32("runtime lhs")?,
                rhs: r.read_u32("runtime rhs")?,
            },
            Opcode::RuntimeTernary => BytecodeInstruction::RuntimeTernary {
                function: r.read_u64("runtime function ID")?,
                dst: r.read_u32("runtime destination")?,
                a: r.read_u32("runtime a")?,
                b: r.read_u32("runtime b")?,
                c: r.read_u32("runtime c")?,
            },
            Opcode::RuntimeQuaternary => BytecodeInstruction::RuntimeQuaternary {
                function: r.read_u64("runtime function ID")?,
                dst: r.read_u32("runtime destination")?,
                a: r.read_u32("runtime a")?,
                b: r.read_u32("runtime b")?,
                c: r.read_u32("runtime c")?,
                d: r.read_u32("runtime d")?,
            },
            Opcode::RuntimeVariadic => {
                let function = r.read_u64("runtime function ID")?;
                let dst = r.read_u32("runtime destination")?;
                let argument_count = r.read_u32("variadic argument count")?;
                let arguments = parse_register_arguments(
                    &mut r,
                    argument_count,
                    max_variadic_arguments,
                    "variadic argument count",
                )?;
                BytecodeInstruction::RuntimeVariadic {
                    function,
                    dst,
                    arguments,
                }
            }
            Opcode::HostCall => {
                let requirement = r.read_u32("host requirement")?;
                let dst = r.read_u32("host destination")?;
                let argument_count = r.read_u32("host argument count")?;
                let arguments = parse_register_arguments(
                    &mut r,
                    argument_count,
                    max_variadic_arguments,
                    "host argument count",
                )?;
                BytecodeInstruction::HostCall {
                    requirement,
                    dst,
                    arguments,
                }
            }
            Opcode::ResourceRead => BytecodeInstruction::ResourceRead {
                requirement: r.read_u32("resource requirement")?,
                dst: r.read_u32("resource destination")?,
            },
            Opcode::ResourceWrite => BytecodeInstruction::ResourceWrite {
                requirement: r.read_u32("resource requirement")?,
                dst: r.read_u32("resource destination")?,
                src: r.read_u32("resource source")?,
            },
            Opcode::ResourceSend => BytecodeInstruction::ResourceSend {
                requirement: r.read_u32("resource requirement")?,
                dst: r.read_u32("resource destination")?,
                src: r.read_u32("resource source")?,
            },
            Opcode::Return => BytecodeInstruction::Return {
                src: r.read_u32("return source")?,
            },
        };
        instructions.push(instruction);
    }
    if !r.is_empty() {
        return invalid("instruction bytes remain after declared instruction count");
    }
    Ok(instructions)
}

fn parse_dictionary(bytes: &[u8], count: u32) -> MResult<BTreeMap<u64, String>> {
    let mut r = ByteReader::new(bytes);
    let count = checked_item_count(count, "dictionary entry count")?;
    validate_minimum_bytes(count, 12, r.remaining(), "dictionary entry count")?;
    let mut dictionary = BTreeMap::new();
    let mut previous = None;
    for _ in 0..count {
        let id = r.read_u64("dictionary ID")?;
        let name = r.read_string("dictionary name")?;
        if name.is_empty() || hash_str(&name) != id {
            return invalid("dictionary name is empty or does not hash to its ID");
        }
        if previous >= Some(id) || dictionary.insert(id, name).is_some() {
            return invalid("dictionary IDs are duplicate or unsorted");
        }
        previous = Some(id);
    }
    if !r.is_empty() {
        return invalid("dictionary section has trailing bytes");
    }
    Ok(dictionary)
}

fn parse_requirements(bytes: &[u8], count: u32) -> MResult<Vec<ApplicationRequirement>> {
    let mut r = ByteReader::new(bytes);
    let count = checked_item_count(count, "application requirement count")?;
    validate_minimum_bytes(count, 16, r.remaining(), "application requirement count")?;
    let mut requirements = try_vec_with_capacity(count, "application requirements")?;
    for _ in 0..count {
        let kind = r.read_u8("requirement kind")?;
        let intent = r.read_u8("requirement intent")?;
        let delivery = r.read_u8("requirement delivery")?;
        if r.read_u8("requirement flags")? != 0 {
            return invalid("requirement flags must be zero");
        }
        let operation_len = checked_usize(
            u64::from(r.read_u16("requirement operation length")?),
            "requirement operation length",
        )?;
        let context_len = checked_usize(
            u64::from(r.read_u16("requirement context length")?),
            "requirement context length",
        )?;
        let primary_len = checked_usize(
            u64::from(r.read_u32("requirement primary length")?),
            "requirement primary length",
        )?;
        let secondary_len = checked_usize(
            u64::from(r.read_u32("requirement secondary length")?),
            "requirement secondary length",
        )?;
        let string_bytes = operation_len
            .checked_add(context_len)
            .and_then(|length| length.checked_add(primary_len))
            .and_then(|length| length.checked_add(secondary_len))
            .ok_or_else(|| invalid::<()>("requirement string byte length overflow").unwrap_err())?;
        if string_bytes > r.remaining() {
            return invalid("requirement string bytes exceed remaining section");
        }
        let operation = r.read_utf8(operation_len, "requirement operation")?;
        let context_name = r.read_utf8(context_len, "requirement context")?;
        let primary = r.read_utf8(primary_len, "requirement primary")?;
        let secondary = r.read_utf8(secondary_len, "requirement secondary")?;
        let requirement = match kind {
            1 => {
                if intent != 0
                    || delivery != 0
                    || !operation.is_empty()
                    || !context_name.is_empty()
                    || primary.is_empty()
                    || !secondary.is_empty()
                {
                    return invalid("invalid HostFunction requirement fields");
                }
                ApplicationRequirement::HostFunction(ExecutionHostFunctionRequest { name: primary })
            }
            2 => {
                let intent = ResourceIntent::from_u8(intent)
                    .ok_or_else(|| invalid::<()>("unknown resource intent").unwrap_err())?;
                let delivery = ResourceDelivery::from_u8(delivery)
                    .ok_or_else(|| invalid::<()>("unknown resource delivery").unwrap_err())?;
                if primary.is_empty() || operation.is_empty() || context_name.is_empty() {
                    return invalid("resource requirement fields must not be empty");
                }
                let requirement = ApplicationRequirement::Resource(ExecutionResourceRequest {
                    base_uri: primary,
                    path: secondary,
                    context_name,
                    operation,
                    intent,
                    delivery,
                });
                validate_application_requirement(&requirement)?;
                requirement
            }
            _ => return invalid("unknown application requirement kind"),
        };
        requirements.push(requirement);
    }
    if !r.is_empty() {
        return invalid("application requirement section has trailing bytes");
    }
    Ok(requirements)
}

fn validate_instructions(
    instructions: &[BytecodeInstruction],
    header: &BytecodeHeader,
    constant_count: usize,
    requirement_count: usize,
    artifact_present: bool,
) -> MResult<Vec<bool>> {
    if instructions.is_empty() && artifact_present && header.register_count == 0 {
        return Ok(Vec::new());
    }
    let register = |instruction: usize, value: u32| {
        if value < header.register_count {
            Ok(value as usize)
        } else {
            invalid(format!(
                "instruction {instruction} register {value} is out of range"
            ))
        }
    };
    let requirement = |value: u32| {
        let value = checked_item_count(value, "instruction requirement index")?;
        if value < requirement_count {
            Ok(())
        } else {
            invalid("instruction requirement index is out of range")
        }
    };
    let mut initialized = vec![false; header.register_count as usize];
    let mut returns = 0;
    for (index, instruction) in instructions.iter().enumerate() {
        match instruction {
            BytecodeInstruction::ConstLoad { dst, constant } => {
                let destination = register(index, *dst)?;
                let constant = checked_item_count(*constant, "instruction constant index")?;
                if constant >= constant_count {
                    return invalid("instruction constant index is out of range");
                }
                if initialized[destination] {
                    return invalid(format!(
                        "instruction {index} register {dst} is initialized more than once"
                    ));
                }
                initialized[destination] = true;
            }
            BytecodeInstruction::CompositePack {
                dst,
                template,
                children,
            } => {
                let destination = register(index, *dst)?;
                let template = checked_item_count(*template, "CompositePack template index")?;
                if template >= constant_count {
                    return invalid("CompositePack template index is out of range");
                }
                if initialized[destination] {
                    return invalid(format!(
                        "instruction {index} register {dst} is initialized more than once"
                    ));
                }
                for child in children {
                    require_initialized_register(&register, &initialized, index, *child)?;
                }
                initialized[destination] = true;
            }
            BytecodeInstruction::RuntimeNullary { function, dst } => {
                if *function == 0 {
                    return invalid("runtime function ID must be nonzero");
                }
                require_initialized_register(&register, &initialized, index, *dst)?;
            }
            BytecodeInstruction::RuntimeUnary { function, dst, src } => {
                if *function == 0 {
                    return invalid("runtime function ID must be nonzero");
                }
                require_initialized_register(&register, &initialized, index, *dst)?;
                require_initialized_register(&register, &initialized, index, *src)?;
            }
            BytecodeInstruction::RuntimeBinary {
                function,
                dst,
                lhs,
                rhs,
            } => {
                if *function == 0 {
                    return invalid("runtime function ID must be nonzero");
                }
                require_initialized_register(&register, &initialized, index, *dst)?;
                require_initialized_register(&register, &initialized, index, *lhs)?;
                require_initialized_register(&register, &initialized, index, *rhs)?;
            }
            BytecodeInstruction::RuntimeTernary {
                function,
                dst,
                a,
                b,
                c,
            } => {
                if *function == 0 {
                    return invalid("runtime function ID must be nonzero");
                }
                for value in [dst, a, b, c] {
                    require_initialized_register(&register, &initialized, index, *value)?;
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
                if *function == 0 {
                    return invalid("runtime function ID must be nonzero");
                }
                for value in [dst, a, b, c, d] {
                    require_initialized_register(&register, &initialized, index, *value)?;
                }
            }
            BytecodeInstruction::RuntimeVariadic {
                function,
                dst,
                arguments,
            } => {
                if *function == 0 {
                    return invalid("runtime function ID must be nonzero");
                }
                require_initialized_register(&register, &initialized, index, *dst)?;
                for value in arguments {
                    require_initialized_register(&register, &initialized, index, *value)?;
                }
            }
            BytecodeInstruction::HostCall {
                requirement: req,
                dst,
                arguments,
            } => {
                requirement(*req)?;
                require_initialized_register(&register, &initialized, index, *dst)?;
                for value in arguments {
                    require_initialized_register(&register, &initialized, index, *value)?;
                }
            }
            BytecodeInstruction::ResourceRead {
                requirement: req,
                dst,
            } => {
                requirement(*req)?;
                let destination = register(index, *dst)?;
                if initialized[destination] {
                    return invalid(format!(
                        "instruction {index} register {dst} is initialized more than once"
                    ));
                }
                initialized[destination] = true;
            }
            BytecodeInstruction::ResourceWrite {
                requirement: req,
                dst,
                src,
            }
            | BytecodeInstruction::ResourceSend {
                requirement: req,
                dst,
                src,
            } => {
                requirement(*req)?;
                require_initialized_register(&register, &initialized, index, *dst)?;
                require_initialized_register(&register, &initialized, index, *src)?;
            }
            BytecodeInstruction::Return { src } => {
                returns += 1;
                if index + 1 != instructions.len() {
                    return invalid("Return must be the final instruction");
                }
                require_initialized_register(&register, &initialized, index, *src)?;
            }
        }
    }
    if returns != 1 {
        return invalid("bytecode must contain exactly one Return instruction");
    }
    Ok(initialized)
}

fn require_initialized_register(
    register: &impl Fn(usize, u32) -> MResult<usize>,
    initialized: &[bool],
    instruction: usize,
    value: u32,
) -> MResult<()> {
    let register = register(instruction, value)?;
    if initialized[register] {
        Ok(())
    } else {
        invalid(format!(
            "instruction {instruction} register {value} is uninitialized"
        ))
    }
}
