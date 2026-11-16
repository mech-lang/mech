//! Compiler-side machine facts. Neither symbols nor stack reports confer
//! source/layout authority. All sizes come from the actual LLVM Object.
use std::collections::{BTreeMap, BTreeSet};

type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(in crate::native_artifact::program) struct FunctionFacts {
    pub stack_bytes: u64,
    pub code_bytes: u64,
    pub text_offset: u64,
}
#[derive(Debug, serde::Serialize)]
pub(in crate::native_artifact::program) struct MachineFacts {
    pub functions: BTreeMap<String, FunctionFacts>,
    pub code_bytes: u64,
    pub text_bytes: u64,
    pub machine_constant_bytes: u64,
}

#[derive(Debug)]
struct Section {
    segment: String,
    name: String,
    address: u64,
    size: u64,
    offset: usize,
}
struct Symbol {
    name: String,
    section: usize,
    address: u64,
}
struct Object<'a> {
    bytes: &'a [u8],
    sections: Vec<Section>,
    symbols: Vec<Symbol>,
    undefined: BTreeSet<String>,
}

fn u32_at(b: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        b.get(at..at.checked_add(4).ok_or("Mach-O offset overflow")?)
            .ok_or("short Mach-O word")?
            .try_into()
            .unwrap(),
    ))
}
fn u64_at(b: &[u8], at: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(
        b.get(at..at.checked_add(8).ok_or("Mach-O offset overflow")?)
            .ok_or("short Mach-O word")?
            .try_into()
            .unwrap(),
    ))
}
fn range(b: &[u8], at: u64, size: u64) -> Result<&[u8]> {
    let end = at.checked_add(size).ok_or("Mach-O range overflow")?;
    b.get(
        usize::try_from(at).map_err(|_| "Mach-O offset exceeds host")?
            ..usize::try_from(end).map_err(|_| "Mach-O extent exceeds host")?,
    )
    .ok_or_else(|| "Mach-O range outside Object".into())
}
fn name(b: &[u8]) -> Result<String> {
    let end = b.iter().position(|v| *v == 0).unwrap_or(b.len());
    if !b[..end].is_ascii() {
        return Err("non-ASCII Mach-O name".into());
    }
    Ok(std::str::from_utf8(&b[..end]).unwrap().to_owned())
}

impl<'a> Object<'a> {
    fn parse(bytes: &'a [u8]) -> Result<Self> {
        if bytes.len() < 32
            || u32_at(bytes, 0)? != 0xfeedfacf
            || u32_at(bytes, 4)? != 0x0100000c
            || u32_at(bytes, 8)? != 0
            || u32_at(bytes, 12)? != 1
        {
            return Err("requires AArch64 Mach-O relocatable Object".into());
        }
        let end = 32usize
            .checked_add(u32_at(bytes, 20)? as usize)
            .ok_or("Mach-O commands overflow")?;
        if end > bytes.len() {
            return Err("Mach-O commands outside Object".into());
        }
        let mut at = 32usize;
        let mut sections = Vec::new();
        let mut symtab = None;
        for _ in 0..u32_at(bytes, 16)? {
            if at.checked_add(8).ok_or("Mach-O command overflow")? > end {
                return Err("short Mach-O load command".into());
            }
            let command = u32_at(bytes, at)?;
            let size = u32_at(bytes, at + 4)? as usize;
            if size < 8 || at.checked_add(size).ok_or("Mach-O command overflow")? > end {
                return Err("invalid Mach-O load command".into());
            }
            if command == 0x19 {
                if size < 72 {
                    return Err("short Mach-O segment".into());
                }
                let count = u32_at(bytes, at + 64)? as usize;
                if count.checked_mul(80).and_then(|n| n.checked_add(72)) != Some(size) {
                    return Err("invalid Mach-O section table".into());
                }
                for i in 0..count {
                    let p = at + 72 + i * 80;
                    let length = u64_at(bytes, p + 40)?;
                    let flags = u32_at(bytes, p + 64)?;
                    if length != 0 && matches!(flags & 255, 1 | 12 | 18) {
                        return Err("unexpected zero-fill machine section".into());
                    }
                    let offset = u32_at(bytes, p + 48)? as usize;
                    range(bytes, offset as u64, length)?;
                    let address = u64_at(bytes, p + 32)?;
                    address
                        .checked_add(length)
                        .ok_or("machine section address overflow")?;
                    sections.push(Section {
                        name: name(&bytes[p..p + 16])?,
                        segment: name(&bytes[p + 16..p + 32])?,
                        address,
                        size: length,
                        offset,
                    });
                }
            } else if command == 2 {
                if size != 24 || symtab.is_some() {
                    return Err("invalid Mach-O symbol table".into());
                }
                symtab = Some((
                    u32_at(bytes, at + 8)?,
                    u32_at(bytes, at + 12)?,
                    u32_at(bytes, at + 16)?,
                    u32_at(bytes, at + 20)?,
                ));
            }
            at += size;
        }
        if at != end {
            return Err("incomplete Mach-O load commands".into());
        }
        let (offset, count, strings, string_size) = symtab.ok_or("missing Mach-O symbol table")?;
        let table = range(
            bytes,
            offset as u64,
            (count as u64)
                .checked_mul(16)
                .ok_or("symbol table overflow")?,
        )?;
        let strings = range(bytes, strings as u64, string_size as u64)?;
        let mut symbols = Vec::new();
        let mut undefined = BTreeSet::new();
        for entry in table.chunks_exact(16) {
            let index = u32_at(entry, 0)? as usize;
            let tail = strings.get(index..).ok_or("invalid symbol string")?;
            let stop = tail
                .iter()
                .position(|v| *v == 0)
                .ok_or("unterminated symbol string")?;
            let spelling = name(&tail[..stop])?;
            let kind = entry[4];
            if kind & 0xe0 != 0 {
                continue;
            }
            if kind & 0x0e == 0x0e {
                let section = entry[5] as usize;
                if section == 0 || section > sections.len() {
                    return Err("invalid symbol section".into());
                }
                symbols.push(Symbol {
                    name: spelling,
                    section: section - 1,
                    address: u64_at(entry, 8)?,
                });
            } else if kind & 0x0e == 0 && !spelling.is_empty() {
                undefined.insert(spelling);
            } else if kind & 0x0e == 0x0a {
                return Err("unsupported indirect Mach-O symbol".into());
            }
        }
        Ok(Self {
            bytes,
            sections,
            symbols,
            undefined,
        })
    }
    fn section_bytes(&self, s: &Section) -> Result<&'a [u8]> {
        range(self.bytes, s.offset as u64, s.size)
    }
    fn text(&self) -> Result<(usize, &Section)> {
        let mut sections = self
            .sections
            .iter()
            .enumerate()
            .filter(|(_, s)| s.segment == "__TEXT" && s.name == "__text");
        let first = sections.next().ok_or("missing machine text")?;
        if sections.next().is_some() {
            return Err("repeated machine text".into());
        }
        Ok(first)
    }
    fn definitions(&self, names: &[String]) -> Result<BTreeMap<String, u64>> {
        let (text_index, text) = self.text()?;
        let expected = names
            .iter()
            .map(|n| (format!("_{n}"), n))
            .collect::<BTreeMap<_, _>>();
        let mut found = BTreeMap::new();
        for symbol in self.symbols.iter().filter(|s| s.section == text_index) {
            if let Some(name) = expected.get(&symbol.name) {
                if found
                    .insert(
                        (*name).clone(),
                        symbol
                            .address
                            .checked_sub(text.address)
                            .ok_or("machine function before text")?,
                    )
                    .is_some()
                {
                    return Err("duplicate machine function".into());
                }
            } else if symbol.name.starts_with('_') {
                return Err(format!("unreported machine function {}", symbol.name));
            }
        }
        if found.len() != names.len()
            || found.values().collect::<BTreeSet<_>>().len() != names.len()
        {
            return Err("missing or aliased machine definition".into());
        }
        if found.values().any(|v| *v >= text.size) {
            return Err("machine definition outside text".into());
        }
        Ok(found)
    }
}

fn valid_name(name: &str) -> bool {
    let mut b = name.bytes();
    matches!(b.next(),Some(v)if v.is_ascii_alphabetic()||b"_$".contains(&v))
        && b.all(|v| v.is_ascii_alphanumeric() || b"_.$".contains(&v))
}
fn stack_reports(text: &str) -> Result<BTreeMap<String, u64>> {
    let mut result = BTreeMap::new();
    for doc in text.split("\n---") {
        let mut fields = BTreeMap::new();
        let mut sizes = Vec::new();
        for line in doc.lines() {
            if let Some((key, value)) = line.split_once(':') {
                if !line.starts_with(char::is_whitespace)
                    && matches!(key, "Pass" | "Name" | "Function")
                {
                    if fields.insert(key, value.trim()).is_some() {
                        return Err("duplicate LLVM stack-report field".into());
                    }
                }
            }
            if let Some(value) = line.trim().strip_prefix("- NumStackBytes:") {
                sizes.push(
                    value
                        .trim()
                        .trim_matches(['\'', '\"'])
                        .parse::<u64>()
                        .map_err(|_| "invalid LLVM stack size")?,
                );
            }
        }
        if fields.get("Pass") != Some(&"prolog-epilog") || fields.get("Name") != Some(&"StackSize")
        {
            continue;
        }
        let name = fields
            .get("Function")
            .ok_or("LLVM stack record lacks function")?
            .trim_matches(['\'', '\"']);
        if !valid_name(name)
            || sizes.len() != 1
            || result.insert(name.to_owned(), sizes[0]).is_some()
        {
            return Err("missing or duplicate LLVM function stack size".into());
        }
    }
    Ok(result)
}

fn constant(s: &Section) -> bool {
    s.segment == "__TEXT"
        && matches!(
            s.name.as_str(),
            "__const" | "__literal4" | "__literal8" | "__literal16" | "__literal_ptr"
        )
}
fn excluded(s: &Section) -> bool {
    s.segment == "__DWARF" || matches!(s.name.as_str(), "__compact_unwind" | "__eh_frame")
}

impl MachineFacts {
    pub(super) fn read(bytes: &[u8], remarks: &str, names: &[String]) -> Result<Self> {
        if names.is_empty()
            || names.iter().any(|n| !valid_name(n))
            || names.iter().collect::<BTreeSet<_>>().len() != names.len()
        {
            return Err("machine manifest is not a nonempty closed set".into());
        }
        let stacks = stack_reports(remarks)?;
        if stacks.keys().collect::<BTreeSet<_>>() != names.iter().collect::<BTreeSet<_>>() {
            return Err("LLVM stack reports differ from sealed definitions".into());
        }
        let object = Object::parse(bytes)?;
        let definitions = object.definitions(names)?;
        let (_, text) = object.text()?;
        // LLVM may lower a bulk-memory idiom to a native C library primitive.
        // This permits no source library operation or per-op interpreter call.
        if object.undefined.iter().any(|n| {
            !matches!(
                n.as_str(),
                "_memcpy" | "_memmove" | "_memset" | "_memcmp" | "_bzero"
            )
        }) {
            return Err(format!(
                "unexpected LLVM machine imports {:?}",
                object.undefined
            ));
        }
        let mut ordered = definitions.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|(_, v)| **v);
        let mut functions = BTreeMap::new();
        for (i, (name, offset)) in ordered.iter().enumerate() {
            let end = ordered.get(i + 1).map_or(text.size, |(_, v)| **v);
            let size = end
                .checked_sub(**offset)
                .filter(|n| *n > 0)
                .ok_or("invalid LLVM function extent")?;
            functions.insert(
                (*name).clone(),
                FunctionFacts {
                    stack_bytes: stacks[*name],
                    code_bytes: size,
                    text_offset: **offset,
                },
            );
        }
        let mut constants = 0u64;
        for s in &object.sections {
            if s.segment == "__TEXT" && s.name == "__text" {
                continue;
            }
            if constant(s) {
                constants = constants
                    .checked_add(s.size)
                    .ok_or("LLVM constant size overflow")?;
            } else if !excluded(s) && s.size != 0 {
                return Err(format!(
                    "unaccounted LLVM machine section {},{}",
                    s.segment, s.name
                ));
            }
        }
        Ok(Self {
            functions,
            code_bytes: text
                .size
                .checked_add(constants)
                .ok_or("LLVM code size overflow")?,
            text_bytes: text.size,
            machine_constant_bytes: constants,
        })
    }
    pub(super) fn audit_assembled(&self, original: &[u8], assembled: &[u8]) -> Result<()> {
        let a = Object::parse(original)?;
        let b = Object::parse(assembled)?;
        let (_, ta) = a.text()?;
        let (_, tb) = b.text()?;
        if a.section_bytes(ta)? != b.section_bytes(tb)? {
            return Err("assembled LLVM text differs from admitted code".into());
        }
        let names = self.functions.keys().cloned().collect::<Vec<_>>();
        let offsets = b.definitions(&names)?;
        if offsets
            .iter()
            .any(|(n, v)| *v != self.functions[n].text_offset)
        {
            return Err("assembled LLVM function placement differs".into());
        }
        for s in a.sections.iter().filter(|s| constant(s) && s.size > 0) {
            let mut matches = b
                .sections
                .iter()
                .filter(|d| d.segment == s.segment && d.name == s.name);
            let final_section = matches.next().ok_or("assembled LLVM constants missing")?;
            if matches.next().is_some()
                || b.section_bytes(final_section)?.get(..s.size as usize)
                    != Some(a.section_bytes(s)?)
            {
                return Err("assembled LLVM constants differ from admitted prefix".into());
            }
        }
        if !b.undefined.is_subset(&a.undefined) {
            return Err("assembled LLVM Object has unexpected unresolved symbols".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn llvm_machine_refuses_missing_repeated_and_overflowing_frames() {
        let good = "--- !Analysis\nPass: prolog-epilog\nName: StackSize\nFunction: test\nArgs:\n  - NumStackBytes: '0'\n...\n";
        assert_eq!(stack_reports(good).unwrap()["test"], 0);
        assert!(stack_reports(&format!("{good}{good}")).is_err());
        assert!(stack_reports(&good.replace("'0'", "'18446744073709551616'")).is_err());
        assert!(MachineFacts::read(&[], good, &["test".into()]).is_err());
        assert!(MachineFacts::read(&[], good, &["other".into()]).is_err());
    }
}
