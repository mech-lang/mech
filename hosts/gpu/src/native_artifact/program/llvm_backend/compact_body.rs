//! Validate the Apple AArch64 definition and its references before partial linking.
use cranelift_codegen::binemit::Reloc;
use cranelift_module::{FuncOrDataId, ModuleDeclarations, ModuleReloc, ModuleRelocTarget};
use cranelift_object::object::{
    self, Object, ObjectKind, ObjectSection, ObjectSymbol, RelocationFlags, RelocationTarget, macho,
};

type Result<T> = std::result::Result<T, String>;

pub(in crate::native_artifact::program) struct CompactBody {
    pub object: Vec<u8>,
    pub code_bytes: u32,
    pub relocations: Vec<ModuleReloc>,
    pub frame_bytes: u64,
}

pub(super) fn read_body(
    input: Vec<u8>,
    frame_bytes: u64,
    entry: &str,
    declarations: &ModuleDeclarations,
) -> Result<CompactBody> {
    let object = object::read::macho::MachOFile64::<object::LittleEndian>::parse(input.as_slice())
        .map_err(|e| e.to_string())?;
    if object.architecture() != object::Architecture::Aarch64
        || object.kind() != ObjectKind::Relocatable
    {
        return Err("compact LLVM body requires a relocatable AArch64 Object".into());
    }
    let text = object
        .section_by_name("__text")
        .ok_or("compact LLVM body is missing text")?;
    let index = text.index();
    let bytes = text.data().map_err(|e| e.to_string())?;
    if text.segment_name().map_err(|e| e.to_string())? != Some("__TEXT")
        || text.address() != 0
        || bytes.is_empty()
        || bytes.len() % 4 != 0
    {
        return Err("unexpected compact LLVM text layout".into());
    }
    for section in object.sections().filter(|s| s.index() != index) {
        let name = section.name().map_err(|e| e.to_string())?;
        if section.size() != 0 && !matches!(name, "__compact_unwind" | "__eh_frame") {
            return Err(format!("unexpected compact LLVM section {name}"));
        }
    }
    let expected = format!("_{entry}");
    let mut definitions = 0;
    for symbol in object
        .symbols()
        .filter(|s| s.section_index() == Some(index))
    {
        let name = symbol.name().map_err(|e| e.to_string())?;
        if name == expected && symbol.address() == 0 {
            definitions += 1;
        } else if name.starts_with('_') {
            return Err(format!("unexpected compact LLVM definition {name}"));
        }
    }
    if definitions != 1 {
        return Err("compact LLVM entry is absent or repeated".into());
    }
    let mut relocations = Vec::new();
    for (offset, relocation) in text.relocations() {
        let RelocationFlags::MachO {
            r_type,
            r_pcrel,
            r_length: 2,
        } = relocation.flags()
        else {
            return Err("unsupported compact LLVM relocation flags".into());
        };
        let RelocationTarget::Symbol(target) = relocation.target() else {
            return Err("compact LLVM relocation needs a named symbol".into());
        };
        let target = object.symbol_by_index(target).map_err(|e| e.to_string())?;
        if !target.is_undefined() {
            return Err("compact LLVM body has a relocation to another definition".into());
        }
        let name = target.name().map_err(|e| e.to_string())?;
        let declaration = declarations
            .get_name(name.strip_prefix('_').ok_or("compact LLVM symbol prefix")?)
            .ok_or_else(|| format!("undeclared compact LLVM symbol {name}"))?;
        let (kind, name): (Reloc, ModuleRelocTarget) = match (r_type, r_pcrel, declaration) {
            (macho::ARM64_RELOC_BRANCH26, true, FuncOrDataId::Func(id)) => {
                (Reloc::Arm64Call, id.into())
            }
            (macho::ARM64_RELOC_PAGE21, true, FuncOrDataId::Data(id)) => {
                (Reloc::Aarch64AdrPrelPgHi21, id.into())
            }
            (macho::ARM64_RELOC_PAGEOFF12, false, FuncOrDataId::Data(id)) => {
                (Reloc::Aarch64AddAbsLo12Nc, id.into())
            }
            _ => return Err(format!("unsupported compact LLVM relocation for {name}")),
        };
        let at = usize::try_from(offset).map_err(|_| "compact LLVM relocation offset")?;
        let instruction = bytes
            .get(at..at.checked_add(4).ok_or("compact LLVM relocation offset")?)
            .filter(|_| offset % 4 == 0)
            .ok_or("compact LLVM relocation lies outside an instruction")?;
        let instruction = u32::from_le_bytes(instruction.try_into().unwrap());
        // An LLVM Mach-O ADDEND pair has already been decoded by Object.
        // Reject implicit offsets that would need to be read out of the word.
        let zero_field = match kind {
            Reloc::Arm64Call => matches!(instruction, 0x1400_0000 | 0x9400_0000),
            Reloc::Aarch64AdrPrelPgHi21 => instruction & 0x60ff_ffe0 == 0,
            Reloc::Aarch64AddAbsLo12Nc => instruction & 0x003f_fc00 == 0,
            _ => unreachable!(),
        };
        if !zero_field {
            return Err("unexpected implicit offset in compact LLVM relocation".into());
        }
        relocations.push(ModuleReloc {
            offset: u32::try_from(offset).map_err(|_| "compact LLVM relocation offset")?,
            kind,
            name,
            addend: relocation.addend(),
        });
    }
    let code_bytes = u32::try_from(bytes.len()).map_err(|_| "compact LLVM body size")?;
    Ok(CompactBody {
        object: input,
        code_bytes,
        relocations,
        frame_bytes,
    })
}

pub(super) fn verify_linked_unwind(root: &[u8], linked: &[u8]) -> Result<()> {
    let root = object::File::parse(root).map_err(|error| error.to_string())?;
    let linked = object::File::parse(linked).map_err(|error| error.to_string())?;
    if linked.architecture() != object::Architecture::Aarch64
        || linked.format() != object::BinaryFormat::MachO
        || linked.kind() != ObjectKind::Relocatable
    {
        return Err("partial link did not emit a relocatable Apple AArch64 Object".into());
    }
    for name in ["__compact_unwind", "__eh_frame"] {
        if root
            .section_by_name(name)
            .is_some_and(|section| section.size() != 0)
            && linked
                .section_by_name(name)
                .is_none_or(|section| section.size() == 0)
        {
            return Err(format!("partial link dropped LLVM unwind section {name}"));
        }
    }
    Ok(())
}
