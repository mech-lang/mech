use cranelift_codegen::ir::{AbiParam, Block, InstBuilder, types};
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::{CodegenError, Context};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

fn isa(verify: bool) -> std::sync::Arc<dyn TargetIsa> {
    let mut flags = settings::builder();
    flags
        .set("enable_verifier", if verify { "true" } else { "false" })
        .unwrap();
    cranelift_native::builder()
        .unwrap()
        .finish(settings::Flags::new(flags))
        .unwrap()
}

fn function(
    isa: &dyn TargetIsa,
    constant_phi: bool,
    dead_exit: bool,
    malformed: bool,
) -> (Context, Block, Block) {
    let mut context = Context::new();
    context.func.signature.call_conv = isa.default_call_conv();
    context.func.signature.params = vec![AbiParam::new(types::I64); 3];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut builders = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut builders);
    let entry = b.create_block();
    let left = b.create_block();
    let right = b.create_block();
    let join = b.create_block();
    b.append_block_params_for_function_params(entry);
    let result = b.append_block_param(join, types::I64);
    b.switch_to_block(entry);
    let [flag, one, two] = b.block_params(entry).try_into().unwrap();
    b.ins().brif(flag, left, &[], right, &[]);
    b.switch_to_block(left);
    b.ins().jump(join, &[one.into()]);
    b.switch_to_block(right);
    let other = if constant_phi { one } else { two };
    b.ins().jump(join, &[other.into()]);
    b.switch_to_block(join);
    b.ins().return_(if malformed {
        &[]
    } else {
        core::slice::from_ref(&result)
    });
    if dead_exit {
        let dead = b.create_block();
        b.switch_to_block(dead);
        b.ins().jump(join, &[one.into()]);
    }
    b.seal_all_blocks();
    b.finalize();
    context.flowgraph();
    (context, left, right)
}

fn compare_graphs(a: &Context, b: &Context) {
    assert_eq!(a.domtree.cfg_postorder(), b.domtree.cfg_postorder());
    for block in a.func.layout.blocks() {
        assert!(a.cfg.succ_iter(block).eq(b.cfg.succ_iter(block)));
        assert!(a.cfg.pred_iter(block).eq(b.cfg.pred_iter(block)));
        assert_eq!(a.domtree.idom(block), b.domtree.idom(block));
    }
}

fn replace_branch(context: &mut Context, first: Block, second: Block) {
    let entry = context.func.layout.entry_block().unwrap();
    let flag = context.func.dfg.block_params(entry)[0];
    let inst = context.func.layout.last_inst(entry).unwrap();
    context
        .func
        .dfg
        .replace(inst)
        .brif(flag, first, &[], second, &[]);
}

fn diagnostic(err: CodegenError) -> String {
    let CodegenError::Verifier(errors) = err else {
        panic!("expected a verifier error");
    };
    assert!(!errors.is_empty());
    errors.to_string()
}

#[test]
fn fresh_and_precomputed_cleanup_match_separate_passes() {
    for verify in [true, false] {
        let isa = isa(verify);
        for constant_phi in [false, true] {
            for dead_exit in [false, true] {
                let (mut separate, ..) = function(&*isa, constant_phi, dead_exit, false);
                separate.eliminate_unreachable_code(&*isa).unwrap();
                separate.remove_constant_phis(&*isa).unwrap();
                let (mut fresh, ..) = function(&*isa, constant_phi, dead_exit, false);
                fresh.compute_flowgraph_and_cleanup(&*isa).unwrap();
                let (mut precomputed, ..) = function(&*isa, constant_phi, dead_exit, false);
                precomputed
                    .prune_unreachable_and_remove_constant_phis(&*isa)
                    .unwrap();
                for context in [&fresh, &precomputed] {
                    context.verify(&*isa).unwrap();
                    let layout = context.func.layout.blocks().collect::<Vec<_>>();
                    assert_eq!(layout.len(), 4);
                    assert_eq!(
                        context.func.dfg.block_params(layout[3]).len(),
                        usize::from(!constant_phi)
                    );
                    assert_eq!(
                        context.func.display().to_string(),
                        separate.func.display().to_string()
                    );
                    compare_graphs(context, &separate);
                }
            }
        }
    }
}

#[test]
fn initial_verification_catches_malformed_returns_before_phi_edits() {
    let isa = isa(true);
    for dead_exit in [false, true] {
        let (mut separate, ..) = function(&*isa, true, dead_exit, true);
        let (mut fresh, ..) = function(&*isa, true, dead_exit, true);
        let expect = diagnostic(separate.eliminate_unreachable_code(&*isa).unwrap_err());
        let got = diagnostic(fresh.compute_flowgraph_and_cleanup(&*isa).unwrap_err());
        assert_eq!(got, expect);
        assert_eq!(
            fresh.func.display().to_string(),
            separate.func.display().to_string()
        );
    }
}

#[test]
fn precomputed_cleanup_still_rejects_stale_successor_order() {
    let isa = isa(true);
    let (mut stale, left, right) = function(&*isa, false, false, false);
    replace_branch(&mut stale, right, left);
    let (mut separate, left, right) = function(&*isa, false, false, false);
    replace_branch(&mut separate, right, left);
    let expect = diagnostic(separate.eliminate_unreachable_code(&*isa).unwrap_err());
    assert!(expect.contains("postorder"), "{expect}");
    assert_eq!(
        diagnostic(
            stale
                .prune_unreachable_and_remove_constant_phis(&*isa)
                .unwrap_err()
        ),
        expect
    );

    stale.compute_flowgraph_and_cleanup(&*isa).unwrap();
    separate.flowgraph();
    separate.eliminate_unreachable_code(&*isa).unwrap();
    separate.remove_constant_phis(&*isa).unwrap();
    compare_graphs(&stale, &separate);
    stale.verify(&*isa).unwrap();
}

#[test]
fn standalone_phi_pass_still_verifies() {
    let isa = isa(true);
    let (mut standalone, ..) = function(&*isa, false, false, true);
    let (mut separate, ..) = function(&*isa, false, false, true);
    let expect = diagnostic(separate.eliminate_unreachable_code(&*isa).unwrap_err());
    assert_eq!(
        diagnostic(standalone.remove_constant_phis(&*isa).unwrap_err()),
        expect
    );
}
