//! Compare the combined validation boundary with the original two calls.
use cranelift_codegen::CodegenResult;
use cranelift_codegen::Context;
use cranelift_codegen::ir::{AbiParam, InstBuilder, types};
use cranelift_codegen::isa::TargetIsa;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Module, default_libcall_names};

fn fixture(module: &JITModule, pruned: bool, phi: bool, invalid: bool) -> Context {
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64)];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut frontend = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut frontend);
    let entry = b.create_block();
    let left = b.create_block();
    let right = b.create_block();
    let join = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.append_block_param(join, types::I64);
    b.switch_to_block(entry);
    let input = b.block_params(entry)[0];
    b.ins().brif(input, left, &[], right, &[]);
    b.switch_to_block(left);
    let value = if phi {
        input
    } else {
        b.ins().iconst(types::I64, 7)
    };
    b.ins().jump(join, &[value.into()]);
    b.switch_to_block(right);
    let value = if phi {
        input
    } else {
        b.ins().iconst(types::I64, 11)
    };
    b.ins().jump(join, &[value.into()]);
    b.switch_to_block(join);
    let result = if invalid {
        b.ins().iconst(types::I32, 1)
    } else {
        b.block_params(join)[0]
    };
    b.ins().return_(&[result]);
    if pruned {
        let dead = b.create_block();
        b.switch_to_block(dead);
        let value = b.ins().iconst(types::I64, 99);
        b.ins().return_(&[value]);
    }
    b.seal_all_blocks();
    b.finalize();
    context
}

fn original(context: &mut Context, isa: &dyn TargetIsa) -> CodegenResult<()> {
    context.compute_flowgraph_and_cleanup(isa)?;
    context.verify(isa)?;
    Ok(())
}

#[test]
fn combined_cleanup_preserves_ir_graph_checks_and_disabled_verifier_fallback() {
    for verifier in ["true", "false"] {
        let module = JITModule::new(
            JITBuilder::with_flags(&[("enable_verifier", verifier)], default_libcall_names())
                .unwrap(),
        );
        for pruned in [false, true] {
            for phi in [false, true] {
                for invalid in [false, true] {
                    let mut reference = fixture(&module, pruned, phi, invalid);
                    let mut combined = Context::for_function(reference.func.clone());
                    let expected = original(&mut reference, module.isa());
                    let actual = combined.compute_flowgraph_cleanup_and_verify(module.isa());
                    let case = format!(
                        "verifier={verifier}, pruned={pruned}, phi={phi}, invalid={invalid}"
                    );
                    assert_eq!(format!("{actual:?}"), format!("{expected:?}"), "{case}");
                    assert_eq!(actual.is_err(), invalid, "{case}");
                    assert_eq!(
                        combined.func.display().to_string(),
                        reference.func.display().to_string(),
                        "{case}"
                    );
                    if !invalid {
                        assert_eq!(combined.func.layout.blocks().count(), 4, "{case}");
                        let parameters: usize = combined
                            .func
                            .layout
                            .blocks()
                            .map(|block| combined.func.dfg.block_params(block).len())
                            .sum();
                        assert_eq!(parameters, if phi { 1 } else { 2 }, "{case}");
                        reference.verify(module.isa()).unwrap();
                        combined.verify(module.isa()).unwrap();
                    }
                }
            }
        }
    }
}

#[test]
fn full_context_verifier_still_rejects_stale_cfg_and_dominator_caches() {
    for verifier in ["true", "false"] {
        let module = JITModule::new(
            JITBuilder::with_flags(&[("enable_verifier", verifier)], default_libcall_names())
                .unwrap(),
        );
        let mut context = fixture(&module, false, false, false);
        context.flowgraph();
        context.verify(module.isa()).unwrap();
        let blocks = context.func.layout.blocks().collect::<Vec<_>>();
        let last = context.func.layout.last_inst(blocks[0]).unwrap();
        let input = context.func.dfg.block_params(blocks[0])[0];
        context
            .func
            .dfg
            .replace(last)
            .jump(blocks[3], &[input.into()]);
        // The replacement is well-typed. Both caches still describe the old
        // diamond, so the independent integrity checks must reject it.
        assert!(
            context.verify(module.isa()).is_err(),
            "cfg, verifier={verifier}"
        );
        context.compute_cfg();
        // The CFG now matches, while the old dominator/postorder cache still
        // includes the two unreachable diamond arms.
        assert!(
            context.verify(module.isa()).is_err(),
            "domtree, verifier={verifier}"
        );
        context.compute_domtree();
        context.verify(module.isa()).unwrap();
    }
}
