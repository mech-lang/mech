use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[test]
fn unchanged_vector_minimum_skips_index_reconstruction_and_preserves_order() {
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("enable_verifier", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let mut functions = Vec::new();
    for defer in [false, true] {
        let mut context = module.make_context();
        context.set_disasm(true);
        context.func.signature.params = vec![AbiParam::new(types::I64); 3];
        context.func.signature.returns = vec![AbiParam::new(types::I32)];
        let mut builder = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let [input, positions, output] = b.block_params(entry)[..] else {
            unreachable!()
        };
        let costs = b.ins().load(types::I32X4, MemFlags::new(), input, 0);
        let best = b.ins().load(types::I32, MemFlags::new(), input, 16);
        let positions = b.ins().load(types::I32X4, MemFlags::new(), positions, 0);
        let missing = b.ins().iconst(types::I32, -1);
        let (minimum, winner) = super::winner::emit(&mut b, costs, positions, best, missing, defer);
        b.ins().store(MemFlags::new(), minimum, output, 0);
        b.ins().store(MemFlags::new(), winner, output, 4);
        b.ins().return_(&[winner]);
        b.seal_all_blocks();
        b.finalize();
        let id = module
            .declare_function(
                &format!("strict_vector_winner_{defer}"),
                Linkage::Local,
                &context.func.signature,
            )
            .unwrap();
        module.define_function(id, &mut context).unwrap();
        if defer {
            let assembly = context.compiled_code().unwrap().vcode.as_ref().unwrap();
            assert!(
                assembly.split_whitespace().any(|op| {
                    op.starts_with("b.") || ["cbz", "cbnz", "tbz", "tbnz"].contains(&op)
                }),
                "winner reconstruction must retain a conditional route: {assembly}"
            );
        }
        functions.push(id);
    }
    module.finalize_definitions().unwrap();
    type Entry = unsafe extern "C" fn(*const u32, *const u32, *mut u32) -> u32;
    let calls = functions
        .into_iter()
        .map(|id| {
            // SAFETY: the live finalized module owns the exact declared C entry.
            unsafe { std::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) }
        })
        .collect::<Vec<_>>();
    let mut seed = 0x68f7_2d9a_0123_4567u64;
    for sample in 0..4096 {
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed as u32
        };
        let mut best = next();
        let mut raw = std::array::from_fn::<_, 4, _>(|_| next());
        let mut positions = std::array::from_fn::<_, 4, _>(|_| next() % 65535);
        match sample % 8 {
            0 => {
                best = u32::MAX;
                raw = [u32::MAX; 4];
            }
            1 => {
                best = 0;
                raw = [0; 4];
            }
            2 => {
                best = 0x8000_0000;
                raw = [0x8000_0000, 0x8000_0001, u32::MAX, 0x8000_0000];
            }
            3 => {
                best = 0x8000_0000;
                raw = [0x7fff_ffff, 0x8000_0000, 0x7fff_ffff, u32::MAX];
                positions = [103, 1, 2, 60];
            }
            4 => {
                best = 8;
                raw = [7, 7, 8, 7];
                positions = [21, 18, 0, 3];
            }
            5 => {
                best = u32::MAX;
                raw = [u32::MAX - 1, u32::MAX, u32::MAX, u32::MAX - 1];
            }
            _ => {}
        }
        let mut input = [0u32; 5];
        for lane in 0..4 {
            input[lane] = raw[lane].min(best);
            if raw[lane] >= best {
                positions[lane] = u32::MAX;
            }
        }
        input[4] = best;
        let minimum = *input[..4].iter().min().unwrap();
        let winner = if minimum < best {
            (0..4)
                .filter(|&lane| input[lane] == minimum)
                .map(|lane| positions[lane])
                .min()
                .unwrap()
        } else {
            u32::MAX
        };
        let original = (input, positions);
        for call in &calls {
            let mut output = [0xa5a5_a5a5; 5];
            // SAFETY: both immutable inputs cover every declared read, the
            // disjoint output covers both writes, and all storage stays live.
            let actual = unsafe {
                call(
                    input.as_ptr(),
                    positions.as_ptr(),
                    output.as_mut_ptr().add(1),
                )
            };
            assert_eq!(actual, winner, "sample {sample}");
            assert_eq!(
                output,
                [0xa5a5_a5a5, minimum, winner, 0xa5a5_a5a5, 0xa5a5_a5a5]
            );
            assert_eq!((input, positions), original);
        }
    }
}
