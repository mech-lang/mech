use super::*;
use crate::native_artifact::program::runtime::maps::utf8_read::{COLD, COMPLETE, MISSING};
use cranelift_codegen::ir::{AbiParam, Opcode};
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{Linkage, Module, default_libcall_names};

struct Kernel {
    entry: unsafe extern "C" fn(*mut Frame) -> u64,
    _module: cranelift_jit::JITModule,
}
impl Kernel {
    fn new(prepaid: bool, vectorized: bool) -> Self {
        Self::with_order(prepaid, vectorized, false)
    }
    fn with_order(prepaid: bool, vectorized: bool, physical_order: bool) -> Self {
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
            .declare_function("utf8_lookup_work", Linkage::Local, &context.func.signature)
            .unwrap();
        let mut fc = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let frame = b.block_params(entry)[0];
            let complete = b.create_block();
            let cold = b.create_block();
            let missing = b.create_block();
            emit(
                &mut b,
                frame,
                complete,
                cold,
                missing,
                prepaid,
                vectorized,
                physical_order,
            );
            for (block, status) in [(complete, COMPLETE), (cold, COLD), (missing, MISSING)] {
                b.switch_to_block(block);
                let result = b.ins().iconst(types::I64, status as i64);
                b.ins().return_(&[result]);
            }
            b.seal_all_blocks();
            b.finalize();
        }
        let mut vectors = 0;
        for block in context.func.layout.blocks() {
            for inst in context.func.layout.block_insts(block) {
                let opcode = context.func.dfg.insts[inst].opcode();
                assert!(!matches!(
                    opcode,
                    Opcode::Call
                        | Opcode::CallIndirect
                        | Opcode::ReturnCall
                        | Opcode::ReturnCallIndirect
                ));
                vectors += usize::from(opcode == Opcode::VanyTrue);
            }
        }
        assert_eq!(vectors, usize::from(vectorized));
        module.define_function(id, &mut context).unwrap();
        module.finalize_definitions().unwrap();
        // SAFETY: the declared signature is one frame pointer returning u64.
        let entry = unsafe { std::mem::transmute(module.get_finalized_function(id)) };
        Self {
            entry,
            _module: module,
        }
    }
    fn run(&self, frame: &mut Frame) -> u64 {
        // SAFETY: each test retains every source/table/output allocation for
        // the synchronous call. The native body contains no call or escape.
        unsafe { (self.entry)(frame) }
    }
}
fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(HASH_OFFSET, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(HASH_PRIME)
    })
}
struct Fixture {
    frame: Frame,
    output: Vec<u64>,
    _slots: Vec<Slot>,
    _cells: Vec<Cell>,
    _entries: Vec<String>,
    _keys: Vec<String>,
}
fn fixture(entries: Vec<String>, keys: Vec<String>, rows: u64, columns: u64) -> Fixture {
    let capacity = (entries.len() * 2).max(4).next_power_of_two();
    let mut slots = vec![Slot::default(); capacity];
    for (i, key) in entries.iter().enumerate() {
        let hashed = hash(key.as_bytes());
        let mut at = hashed as usize & (capacity - 1);
        while slots[at].occupied != 0 {
            at = (at + 1) & (capacity - 1);
        }
        slots[at] = Slot {
            address: key.as_ptr() as u64,
            bytes: key.len() as u64,
            value: i as u64 + 100,
            hash: hashed,
            occupied: 1,
        };
    }
    let cells = keys
        .iter()
        .map(|key| Cell {
            address: key.as_ptr() as u64,
            bytes: key.len() as u64,
        })
        .collect::<Vec<_>>();
    let mut output = vec![u64::MAX; keys.len()];
    let frame = Frame {
        slots: slots.as_ptr() as u64,
        capacity: capacity as u64,
        cells: cells.as_ptr() as u64,
        count: keys.len() as u64,
        rows,
        columns,
        output: output.as_mut_ptr() as u64,
        stored_bytes: entries.iter().map(|s| s.len() as u64).sum(),
        fuel: u64::MAX,
        comparison: u64::MAX,
        epoch: 7,
        issued: 7,
        next: 0,
    };
    Fixture {
        frame,
        output,
        _slots: slots,
        _cells: cells,
        _entries: entries,
        _keys: keys,
    }
}

#[test]
fn native_utf8_work_scalar_and_simd_preserve_full_keys_geometry_and_misses() {
    let mut keys = vec![
        String::new(),
        "\0".into(),
        "a".repeat(15),
        "b".repeat(16),
        "c".repeat(17),
        "雪\0é".repeat(9),
        "d".repeat(31),
        "e".repeat(32),
        "f".repeat(33),
        "z".repeat(8193),
        "👩‍🔬".into(),
        "same-prefix-different-tail".into(),
    ];
    // Deliberately put several distinct keys into the same initial hash bucket.
    let colliders = (0..10000)
        .map(|i| format!("collision-{i}"))
        .filter(|s| hash(s.as_bytes()) & 63 == 0)
        .take(8)
        .collect::<Vec<_>>();
    assert_eq!(colliders.len(), 8);
    keys.extend(colliders);
    for prepaid in [false, true] {
        for vectorized in [false, true] {
            let kernel = Kernel::new(prepaid, vectorized);
            let mut f = fixture(keys.clone(), keys.clone(), 4, 5);
            assert_eq!(kernel.run(&mut f.frame), COMPLETE);
            assert_eq!(f.frame.next, 20);
            assert_eq!(
                f.output,
                (0..20)
                    .map(|i| 100 + (i % 4) * 5 + i / 4)
                    .collect::<Vec<_>>()
            );
            let mut missing = fixture(
                vec!["yes".into()],
                vec!["yes".into(), "absent".into()],
                1,
                2,
            );
            assert_eq!(kernel.run(&mut missing.frame), MISSING);
            assert_eq!(missing.frame.next, 1);
            assert_eq!(missing.output, [100, u64::MAX]);
            let mut empty = fixture(vec![], vec![], 0, 9);
            assert_eq!(kernel.run(&mut empty.frame), COMPLETE);
            assert_eq!(empty.frame.next, 0);
        }
    }
}

#[test]
fn native_utf8_work_finite_physical_allowance_and_descriptor_guards() {
    for vectorized in [false, true] {
        let kernel = Kernel::new(false, vectorized);
        let mut f = fixture(vec!["key".into()], vec!["key".into()], 1, 1);
        let comparison = f.frame.capacity * 4 + 32 + f.frame.stored_bytes + 3;
        let compute = comparison * 64;
        f.frame.fuel = compute - 1;
        f.frame.comparison = comparison;
        assert_eq!(kernel.run(&mut f.frame), COLD);
        assert_eq!(
            (f.frame.next, f.frame.fuel, f.frame.comparison),
            (0, compute - 1, comparison)
        );
        assert_eq!(f.output, [u64::MAX]);
        f.frame.fuel = compute;
        f.frame.comparison = comparison - 1;
        assert_eq!(kernel.run(&mut f.frame), COLD);
        assert_eq!(f.frame.next, 0);
        f.frame.comparison = comparison;
        assert_eq!(kernel.run(&mut f.frame), COMPLETE);
        assert_eq!((f.frame.fuel, f.frame.comparison), (0, 0));
        assert_eq!(f.output, [100]);
        for bad in 0..6 {
            let mut f = fixture(vec!["key".into()], vec!["key".into()], 1, 1);
            match bad {
                0 => f.frame.issued = 0,
                1 => f.frame.issued += 1,
                2 => f.frame.capacity = 3,
                3 => f.frame.rows = 2,
                4 => {
                    f.frame.rows = u64::MAX;
                    f.frame.columns = 2;
                }
                5 => f.frame.next = 2,
                _ => unreachable!(),
            }
            assert_eq!(kernel.run(&mut f.frame), COLD);
            assert_eq!(f.output, [u64::MAX]);
        }
        // No empty slot: every bounded probe must eventually decline.
        let mut f = fixture(vec!["key".into()], vec!["missing".into()], 1, 1);
        let first = f._slots.iter().copied().find(|s| s.occupied != 0).unwrap();
        f._slots.fill(first);
        assert_eq!(kernel.run(&mut f.frame), COLD);
        assert_eq!(f.frame.next, 0);
        assert_eq!(f.output, [u64::MAX]);
    }
}

#[test]
fn native_utf8_collection_order_uses_dense_cells_and_rejects_foreign_geometry() {
    assert_eq!(size_of::<Cell>(), 16);
    let keys = vec![
        String::new(),
        "x".repeat(3),
        "y".repeat(4),
        "z".repeat(7),
        "a".repeat(8),
        "b".repeat(15),
        "c".repeat(16),
        "雪é\0".repeat(3),
    ];
    for prepaid in [false, true] {
        for vectorized in [false, true] {
            let kernel = Kernel::with_order(prepaid, vectorized, true);
            let mut data = fixture(keys.clone(), keys.clone(), 1, keys.len() as u64);
            assert_eq!(kernel.run(&mut data.frame), COMPLETE);
            assert_eq!(
                data.output,
                (100..100 + keys.len() as u64).collect::<Vec<_>>()
            );
            let mut foreign = fixture(keys.clone(), keys.clone(), 2, 4);
            foreign.frame.cells = 0; // must decline before any payload access
            assert_eq!(kernel.run(&mut foreign.frame), COLD);
            assert_eq!(foreign.frame.next, 0);
            assert!(foreign.output.iter().all(|&value| value == u64::MAX));
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn native_utf8_work_simd_tail_never_reads_past_an_admitted_key() {
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
        key: *const u8,
    }
    impl Guarded {
        fn new(bytes: &[u8]) -> Self {
            // SAFETY: getpagesize has no pointer arguments or mutable state.
            let page = unsafe { getpagesize() } as usize;
            assert!(bytes.len() < page);
            let anonymous = if cfg!(target_os = "macos") {
                0x1000
            } else {
                0x20
            };
            // SAFETY: reserve two fresh pages; retain the exact mapping extent.
            let base = unsafe { mmap(std::ptr::null_mut(), page * 2, 3, 2 | anonymous, -1, 0) };
            assert_ne!(base as usize, usize::MAX);
            // SAFETY: both addresses belong to the fresh writable mapping. The
            // second page is made inaccessible, including an empty key's ptr.
            let key = unsafe {
                let guard = base.cast::<u8>().add(page);
                let key = guard.sub(bytes.len());
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), key, bytes.len());
                assert_eq!(mprotect(guard.cast(), page, 0), 0);
                key
            };
            Self {
                base,
                pages: page * 2,
                key,
            }
        }
    }
    impl Drop for Guarded {
        fn drop(&mut self) {
            // SAFETY: this owner retains the same complete mapping until now.
            assert_eq!(unsafe { munmap(self.base, self.pages) }, 0);
        }
    }
    let kernel = Kernel::new(true, true);
    for len in [0, 1, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65] {
        let bytes = vec![b'a'; len];
        let query = Guarded::new(&bytes);
        let stored = Guarded::new(&bytes);
        let mut slots = vec![Slot::default(); 4];
        let hashed = hash(&bytes);
        slots[hashed as usize & 3] = Slot {
            address: stored.key as u64,
            bytes: len as u64,
            value: 731,
            hash: hashed,
            occupied: 1,
        };
        // Two identical queries also prove the completed cursor is per visit,
        // rather than a reused key-owner result or one-shot producer permit.
        let cell = Cell {
            address: query.key as u64,
            bytes: len as u64,
        };
        let cells = [cell, cell];
        let mut output = [u64::MAX; 2];
        let mut frame = Frame {
            slots: slots.as_ptr() as u64,
            capacity: 4,
            cells: cells.as_ptr() as u64,
            count: 2,
            rows: 2,
            columns: 1,
            output: output.as_mut_ptr() as u64,
            stored_bytes: len as u64,
            fuel: 0,
            comparison: 0,
            epoch: 9,
            issued: 9,
            next: 0,
        };
        assert_eq!(kernel.run(&mut frame), COMPLETE);
        assert_eq!(output, [731, 731]);
        assert_eq!(frame.next, 2);
    }
}
