use super::*;

unsafe extern "C" fn never_entered(_: *mut c_void) -> u64 {
    panic!("header tests cannot execute code")
}

#[test]
fn incompatible_module_headers_are_rejected_before_following_relocations() {
    let image = [1u8];
    let identity = [0u8; NATIVE_PROGRAM_LINK_IDENTITY_BYTES];
    let binding = [0u8; NATIVE_PROGRAM_IMAGE_BINDING_BYTES];
    let fresh = || NativeProgramModule {
        magic: MAGIC,
        version: VERSION,
        pointer_bits: 64,
        image: image.as_ptr(),
        image_len: image.len() as u64,
        identity: &identity,
        binding: &binding,
        entry: never_entered,
    };
    assert!(fresh().valid_header());
    for case in 0..10 {
        let mut module = fresh();
        match case {
            0 => module.magic[0] ^= 1,
            1 => module.magic[15] ^= 1,
            2 => module.version = 0,
            3 => module.version += 1,
            4 => module.pointer_bits = 32,
            5 => module.image_len = 0,
            6 => module.image_len = isize::MAX as u64 + 1,
            7 => module.image = std::ptr::null(),
            8 => module.identity = std::ptr::null(),
            9 => module.binding = std::ptr::null(),
            _ => unreachable!(),
        }
        // No relocation is dereferenced; no fabricated metadata gains trust.
        assert!(!module.valid_header(), "case {case}");
    }
}
