//! Check the Apple AArch64 cold-return layout with and without authentication.

#![cfg(target_arch = "aarch64")]

use cranelift_codegen::{
    CompiledCode, Context,
    ir::{AbiParam, Function, InstBuilder, types::I64},
    isa,
    settings::{self, Configurable},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

fn compile(share: bool, authenticate: bool, cfg_info: bool) -> CompiledCode {
    let mut flags = settings::builder();
    flags.enable("preserve_frame_pointers").unwrap();
    flags.enable("unwind_info").unwrap();
    flags
        .set(
            "machine_code_cfg_info",
            if cfg_info { "true" } else { "false" },
        )
        .unwrap();
    let mut target = isa::lookup("aarch64-apple-darwin".parse().unwrap()).unwrap();
    if authenticate {
        target.enable("has_pauth").unwrap();
        target.enable("sign_return_address").unwrap();
        target.enable("sign_return_address_with_bkey").unwrap();
    }
    let isa = target.finish(settings::Flags::new(flags)).unwrap();

    let mut func = Function::new();
    func.signature.call_conv = isa.default_call_conv();
    func.signature.params = vec![AbiParam::new(I64); 2];
    func.signature.returns = vec![AbiParam::new(I64); 2];
    func.share_cold_epilogues = share;
    let mut builder_context = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut func, &mut builder_context);
    let entry = builder.create_block();
    let success = builder.create_block();
    let dispatch = builder.create_block();
    let first_error = builder.create_block();
    let second_error = builder.create_block();
    builder.set_cold_block(first_error);
    builder.set_cold_block(second_error);
    builder.append_block_params_for_function_params(entry);
    builder.switch_to_block(entry);
    let test = builder.block_params(entry)[0];
    let which_error = builder.block_params(entry)[1];
    builder.ins().brif(test, success, &[], dispatch, &[]);
    builder.switch_to_block(success);
    let zero = builder.ins().iconst(I64, 0);
    builder.ins().return_(&[zero, zero]);
    builder.switch_to_block(dispatch);
    builder
        .ins()
        .brif(which_error, first_error, &[], second_error, &[]);
    for (block, status, origin) in [(first_error, 1, 17), (second_error, 2, 34)] {
        builder.switch_to_block(block);
        let status = builder.ins().iconst(I64, status);
        let origin = builder.ins().iconst(I64, origin);
        builder.ins().return_(&[status, origin]);
    }
    builder.seal_all_blocks();
    builder.finalize();
    let mut context = Context::for_function(func);
    context.compile(&*isa, &mut Default::default()).unwrap();
    context.take_compiled_code().unwrap()
}

fn words(code: &CompiledCode) -> Vec<u32> {
    code.code_buffer()
        .chunks_exact(4)
        .map(|word| u32::from_le_bytes(word.try_into().unwrap()))
        .collect()
}

#[test]
fn cold_returns_share_restore_and_keep_the_success_return() {
    // ldp x29, x30, [sp], #16
    const RESTORE_FRAME: u32 = 0xa8c17bfd;

    for (authenticate, ret) in [(false, 0xd65f03c0), (true, 0xd65f0fff)] {
        let control = compile(false, authenticate, false);
        let candidate = compile(true, authenticate, false);
        assert_eq!(control.buffer.unwind_info, candidate.buffer.unwind_info);

        let control = words(&control);
        let candidate = words(&candidate);
        let returns = |words: &[u32]| {
            words
                .iter()
                .enumerate()
                .filter_map(|(index, &word)| (word == ret).then_some(index))
                .collect::<Vec<_>>()
        };
        let old_returns = returns(&control);
        let new_returns = returns(&candidate);
        assert_eq!(old_returns.len(), 3);
        assert_eq!(new_returns.len(), 2);
        assert_eq!(new_returns[0], old_returns[0]);
        let tail = new_returns[1] - 1;
        assert_eq!(candidate[tail], RESTORE_FRAME);
        assert!(candidate.iter().enumerate().any(|(index, &word)| {
            // B immediate with a signed 26-bit displacement in words.
            word >> 26 == 0b000101 && index as i32 + ((word << 6) as i32 >> 6) == tail as i32
        }));
        assert_eq!(control.len() - candidate.len(), 1);
    }
}

#[test]
fn machine_cfg_info_keeps_the_original_return_blocks() {
    let control = compile(false, false, true);
    let candidate = compile(true, false, true);
    assert_eq!(control.code_buffer(), candidate.code_buffer());
    assert_eq!(control.bb_starts, candidate.bb_starts);
    assert_eq!(control.bb_edges, candidate.bb_edges);
}
