use super::*;
use cranelift_codegen::ir::{AbiParam, Opcode};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{Linkage, Module, default_libcall_names};
struct Kernel {
    call: unsafe extern "C" fn(*const u8, *mut u8),
    _module: cranelift_jit::JITModule,
}
impl Kernel {
    fn new(bytes: usize, width: u8) -> Self {
        let builder = cranelift_jit::JITBuilder::with_flags(
            &[("opt_level", "speed")],
            default_libcall_names(),
        )
        .unwrap();
        let mut module = cranelift_jit::JITModule::new(builder);
        let mut cx = module.make_context();
        cx.func.signature.params = vec![AbiParam::new(types::I64); 2];
        let id = module
            .declare_function("native_bit_copy", Linkage::Local, &cx.func.signature)
            .unwrap();
        let mut fc = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut cx.func, &mut fc);
            let root = b.create_block();
            b.append_block_params_for_function_params(root);
            b.switch_to_block(root);
            let args = b.block_params(root).to_vec();
            emit(&mut b, args[0], args[1], bytes as u64, width).unwrap();
            b.ins().return_(&[]);
            b.seal_all_blocks();
            b.finalize();
        }
        for block in cx.func.layout.blocks() {
            for inst in cx.func.layout.block_insts(block) {
                assert!(!matches!(
                    cx.func.dfg.insts[inst].opcode(),
                    Opcode::Call | Opcode::CallIndirect
                ));
            }
        }
        cx.verify(module.isa()).unwrap();
        module.define_function(id, &mut cx).unwrap();
        module.finalize_definitions().unwrap();
        let call = unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        Self {
            call,
            _module: module,
        }
    }
    unsafe fn run(&self, source: *const u8, output: *mut u8) {
        unsafe { (self.call)(source, output) }
    }
}
fn bits(bytes: usize) -> Vec<u8> {
    let words = [
        0u64,
        1,
        1 << 63,
        u64::MAX,
        0x7ff0_0000_0000_0031,
        0x7ff8_0000_0000_0042,
        0xfff8_0000_0000_0064,
        0x7fc0_0042_7f80_0031,
        0x0123_4567_89ab_cdef,
    ];
    let pattern = words
        .into_iter()
        .flat_map(u64::to_le_bytes)
        .collect::<Vec<_>>();
    (0..bytes).map(|at| pattern[at % pattern.len()]).collect()
}
#[test]
fn numeric_bit_copy_preserves_all_widths_bits_unaligned_spans_and_exact_tails() {
    for width in [1, 2, 4, 8] {
        for count in [0, 1, 2, 3, 7, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127] {
            let bytes = count * width as usize;
            let kernel = Kernel::new(bytes, width);
            let source = bits(bytes + 9);
            let mut output = vec![0xa5; bytes + 9];
            let before = source.clone();
            unsafe {
                kernel.run(source.as_ptr().add(3), output.as_mut_ptr().add(1));
            }
            assert_eq!(source, before);
            assert_eq!(&output[1..1 + bytes], &source[3..3 + bytes]);
            assert!(
                output[..1]
                    .iter()
                    .chain(&output[1 + bytes..])
                    .all(|&b| b == 0xa5)
            );
            if bytes == 0 {
                unsafe {
                    kernel.run(core::ptr::null(), core::ptr::null_mut());
                }
            }
        }
    }
}
#[test]
fn numeric_bit_copy_keeps_scalar_unit_order_for_both_partial_overlaps_and_self_copy() {
    for width in [1, 2, 4, 8] {
        for count in [1, 3, 15, 16, 17, 31, 32, 65] {
            let bytes = count * width as usize;
            let kernel = Kernel::new(bytes, width);
            for delta in [0, 1, 3, 7, 8, 15, 16, 17, 63, 64, 65] {
                for reverse in [false, true] {
                    let (from, to) = if reverse {
                        (delta + 1, 1)
                    } else {
                        (1, delta + 1)
                    };
                    let mut actual = bits(bytes + delta + 2);
                    let mut expected = actual.clone();
                    for at in (0..bytes).step_by(width as usize) {
                        let word = expected[from + at..from + at + width as usize].to_vec();
                        expected[to + at..to + at + width as usize].copy_from_slice(&word);
                    }
                    unsafe {
                        kernel.run(actual.as_ptr().add(from), actual.as_mut_ptr().add(to));
                    }
                    assert_eq!(
                        actual, expected,
                        "width{width} count{count} delta{delta} reverse{reverse}"
                    );
                }
            }
        }
    }
}
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn numeric_bit_copy_never_reads_or_writes_past_exact_guarded_spans() {
    use std::ffi::c_void;
    unsafe extern "C" {
        fn getpagesize() -> i32;
        fn mmap(
            at: *mut c_void,
            bytes: usize,
            protection: i32,
            flags: i32,
            fd: i32,
            offset: i64,
        ) -> *mut c_void;
        fn mprotect(at: *mut c_void, bytes: usize, protection: i32) -> i32;
        fn munmap(at: *mut c_void, bytes: usize) -> i32;
    }
    struct Guarded {
        base: *mut c_void,
        pages: usize,
        pointer: *mut u8,
    }
    impl Guarded {
        fn new(bytes: usize) -> Self {
            let page = unsafe { getpagesize() } as usize;
            assert!(bytes < page);
            let anonymous = if cfg!(target_os = "macos") {
                0x1000
            } else {
                0x20
            };
            let base = unsafe { mmap(core::ptr::null_mut(), 2 * page, 3, 2 | anonymous, -1, 0) };
            assert_ne!(base as usize, usize::MAX);
            let pointer = unsafe {
                let guard = base.cast::<u8>().add(page);
                assert_eq!(mprotect(guard.cast(), page, 0), 0);
                guard.sub(bytes)
            };
            Self {
                base,
                pages: 2 * page,
                pointer,
            }
        }
    }
    impl Drop for Guarded {
        fn drop(&mut self) {
            assert_eq!(unsafe { munmap(self.base, self.pages) }, 0);
        }
    }
    for width in [1, 2, 4, 8] {
        for count in [0, 1, 2, 7, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65] {
            let bytes = count * width as usize;
            let kernel = Kernel::new(bytes, width);
            let source = Guarded::new(bytes);
            let output = Guarded::new(bytes);
            let expected = bits(bytes);
            unsafe {
                core::ptr::copy_nonoverlapping(expected.as_ptr(), source.pointer, bytes);
                kernel.run(source.pointer, output.pointer);
                assert_eq!(core::slice::from_raw_parts(output.pointer, bytes), expected);
                assert_eq!(core::slice::from_raw_parts(source.pointer, bytes), expected);
            }
        }
    }
}
