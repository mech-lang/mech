use super::*;
use cranelift_codegen::ir::{AbiParam, Opcode};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{Linkage, Module, default_libcall_names};
struct Kernel {
    entry: unsafe extern "C" fn(*mut Frame) -> u64,
    _module: cranelift_jit::JITModule,
}
impl Kernel {
    fn new(width: u64, vectorized: bool) -> Self {
        let builder = cranelift_jit::JITBuilder::with_flags(
            &[("opt_level", "speed")],
            default_libcall_names(),
        )
        .unwrap();
        let mut module = cranelift_jit::JITModule::new(builder);
        let mut context = module.make_context();
        context
            .func
            .signature
            .params
            .push(AbiParam::new(types::I64));
        context
            .func
            .signature
            .returns
            .push(AbiParam::new(types::I64));
        let id = module
            .declare_function(
                "native_unsigned_fill",
                Linkage::Local,
                &context.func.signature,
            )
            .unwrap();
        let mut fc = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let frame = b.block_params(entry)[0];
            let complete = b.create_block();
            let bad = b.create_block();
            emit(&mut b, frame, width, complete, bad, vectorized);
            for (block, status) in [(complete, 0), (bad, 1)] {
                b.switch_to_block(block);
                let status = b.ins().iconst(types::I64, status);
                b.ins().return_(&[status]);
            }
            b.seal_all_blocks();
            b.finalize();
        }
        let mut splats = 0;
        for block in context.func.layout.blocks() {
            for inst in context.func.layout.block_insts(block) {
                let op = context.func.dfg.insts[inst].opcode();
                assert!(!matches!(
                    op,
                    Opcode::Call
                        | Opcode::CallIndirect
                        | Opcode::ReturnCall
                        | Opcode::ReturnCallIndirect
                ));
                splats += usize::from(op == Opcode::Splat);
            }
        }
        assert_eq!(splats, usize::from(vectorized));
        module.define_function(id, &mut context).unwrap();
        module.finalize_definitions().unwrap();
        // SAFETY: declared signature is exactly one live frame pointer -> u64.
        let entry = unsafe { std::mem::transmute(module.get_finalized_function(id)) };
        Self {
            entry,
            _module: module,
        }
    }
    fn run(&self, frame: &mut Frame) -> u64 {
        // SAFETY: each test pins its output allocation for this synchronous call.
        unsafe { (self.entry)(frame) }
    }
}
fn maximum(width: u64) -> u64 {
    if width == 8 {
        u64::MAX
    } else {
        (1u64 << (width * 8)) - 1
    }
}
fn frame(output: *mut u8, count: usize, width: u64, word: u64) -> Frame {
    Frame {
        output: output as u64,
        count: count as u64,
        capacity_bytes: count as u64 * width,
        word,
        width,
        epoch: 17,
        issued: 17,
        next: 0,
    }
}
#[test]
fn native_fill_scalar_simd_all_unsigned_widths_match_with_unaligned_tails() {
    for width in [1, 2, 4, 8] {
        for vectorized in [false, true] {
            let kernel = Kernel::new(width, vectorized);
            for count in [0, 1, 2, 3, 7, 8, 15, 16, 17, 31, 32, 33, 127] {
                for word in [0, 1, maximum(width), 0x1122_3344_5566_7788 & maximum(width)] {
                    let bytes = count * width as usize;
                    let mut output = vec![0xa5; bytes + 2];
                    let mut f = frame(unsafe { output.as_mut_ptr().add(1) }, count, width, word);
                    assert_eq!(kernel.run(&mut f), 0);
                    assert_eq!(f.next, count as u64);
                    assert_eq!(output[0], 0xa5);
                    assert_eq!(output[bytes + 1], 0xa5);
                    let expected = word.to_le_bytes();
                    for value in output[1..bytes + 1].chunks_exact(width as usize) {
                        assert_eq!(value, &expected[..width as usize]);
                    }
                }
            }
        }
    }
}
#[test]
fn native_fill_invalid_bank_geometry_is_rejected_before_any_store() {
    for width in [1, 2, 4, 8] {
        for vectorized in [false, true] {
            let kernel = Kernel::new(width, vectorized);
            for change in 0..9 {
                let mut output = vec![0xa5; 64 * width as usize];
                let original = output.clone();
                let mut f = frame(output.as_mut_ptr(), 64, width, 1);
                match change {
                    0 => f.issued += 1,
                    1 => f.epoch = 0,
                    2 => f.next = 1,
                    3 => f.capacity_bytes -= 1,
                    4 => f.count = u64::MAX,
                    5 => f.width = 3,
                    6 => f.output = 0,
                    7 => f.output = u64::MAX - 3,
                    8 => {
                        if width == 8 {
                            continue;
                        } else {
                            f.word = maximum(width) + 1
                        }
                    }
                    _ => unreachable!(),
                }
                assert_eq!(kernel.run(&mut f), 1, "width{width} change{change}");
                assert_eq!(output, original);
            }
        }
    }
}
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn native_fill_simd_never_writes_beyond_exact_guarded_capacity() {
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
        output: *mut u8,
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
            let base = unsafe { mmap(std::ptr::null_mut(), page * 2, 3, 2 | anonymous, -1, 0) };
            assert_ne!(base as usize, usize::MAX);
            let output = unsafe {
                let guard = base.cast::<u8>().add(page);
                assert_eq!(mprotect(guard.cast(), page, 0), 0);
                guard.sub(bytes)
            };
            Self {
                base,
                pages: page * 2,
                output,
            }
        }
    }
    impl Drop for Guarded {
        fn drop(&mut self) {
            assert_eq!(unsafe { munmap(self.base, self.pages) }, 0)
        }
    }
    for width in [1, 2, 4, 8] {
        let kernel = Kernel::new(width, true);
        for count in [0, 1, 2, 7, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65] {
            let bytes = count * width as usize;
            let output = Guarded::new(bytes);
            let word = maximum(width);
            let mut f = frame(output.output, count, width, word);
            assert_eq!(kernel.run(&mut f), 0);
            assert_eq!(f.next, count as u64);
            // SAFETY: exactly the writable part of the retained mapping.
            let values = unsafe { std::slice::from_raw_parts(output.output, bytes) };
            assert!(values.iter().all(|&b| b == 0xff));
        }
    }
}
