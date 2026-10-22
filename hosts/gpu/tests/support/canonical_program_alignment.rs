//! Read the actual emitted Mach-O alignment, not a requested flag or metadata.
use mech_gpu::native_artifact::emit_program_object;

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}

fn name(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes.split(|byte| *byte == 0).next().unwrap()).unwrap()
}

#[test]
fn aarch64_program_functions_have_cache_line_stable_object_alignment() {
    for source in [super::ECHO, super::FULL] {
        let (variants, catalog) = super::artifacts(source);
        for artifact in variants {
            let object = emit_program_object(&artifact, &catalog).unwrap();
            let bytes = object.bytes();
            assert_eq!(u32_at(bytes, 0), 0xfeedfacf);
            let mut command = 32;
            let mut sections = 0;
            let mut text_section = None;
            let mut symbols = None;
            for _ in 0..u32_at(bytes, 16) {
                match u32_at(bytes, command) {
                    0x19 => {
                        for index in 0..u32_at(bytes, command + 64) as usize {
                            let section = command + 72 + index * 80;
                            sections += 1;
                            if name(&bytes[section..section + 16]) == "__text" {
                                assert!(u32_at(bytes, section + 52) >= 7);
                                text_section = Some(sections);
                            }
                        }
                    }
                    2 => {
                        symbols = Some((
                            u32_at(bytes, command + 8) as usize,
                            u32_at(bytes, command + 12) as usize,
                            u32_at(bytes, command + 16) as usize,
                        ));
                    }
                    _ => {}
                }
                command += u32_at(bytes, command + 4) as usize;
            }
            let text_section = text_section.unwrap();
            let (symbols, count, strings) = symbols.unwrap();
            let mut generated = Vec::new();
            for index in 0..count {
                let symbol = symbols + index * 16;
                let symbol_name = name(&bytes[strings + u32_at(bytes, symbol) as usize..]);
                if bytes[symbol + 5] as usize == text_section
                    && bytes[symbol + 4] & 0x0e == 0x0e
                    && symbol_name.starts_with("_mech_")
                {
                    assert_eq!(u64_at(bytes, symbol + 8) % 128, 0, "{symbol_name}");
                    generated.push(symbol_name);
                }
            }
            assert!(generated.contains(&"_mech_canonical_program_entry_v1"));
        }
    }
}
