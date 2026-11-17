use cranelift_codegen::{CodegenError, Context};
use cranelift_codegen::ir::{AbiParam, InstBuilder, types};
use cranelift_codegen::isa::TargetIsa;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

fn isa() -> std::sync::Arc<dyn TargetIsa> {
    let mut flags = settings::builder();
    flags.set("enable_verifier", "true").unwrap();
    cranelift_native::builder()
        .unwrap()
        .finish(settings::Flags::new(flags))
        .unwrap()
}

fn function(isa: &dyn TargetIsa, constant_phi: bool, dead_exit: bool, malformed: bool) -> Context {
    let mut context = Context::new();
    context.func.signature.call_conv = isa.default_call_conv();
    context.func.signature.params = vec![AbiParam::new(types::I64); 3];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut builders = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut builders);
    let entry = b.create_block();
    let join = b.create_block();
    b.append_block_params_for_function_params(entry);
    let result = b.append_block_param(join, types::I64);
    b.switch_to_block(entry);
    let [flag, left, right] = b.block_params(entry).try_into().unwrap();
    let otherwise = if constant_phi { left } else { right };
    b.ins()
        .brif(flag, join, &[left.into()], join, &[otherwise.into()]);
    b.switch_to_block(join);
    b.ins().return_(if malformed {
        &[]
    } else {
        core::slice::from_ref(&result)
    });
    if dead_exit {
        let dead = b.create_block();
        b.switch_to_block(dead);
        let one = b.ins().iconst(types::I64, 1);
        b.ins().return_(&[one]);
    }
    b.seal_all_blocks();
    b.finalize();
    context.flowgraph();
    context
}

#[test]
fn changed_and_unchanged_sweeps_match_the_individual_passes() {
    let isa = isa();
    for constant_phi in [false, true] {
        for dead_exit in [false, true] {
            let mut pair = function(&*isa, constant_phi, dead_exit, false);
            let mut separate = function(&*isa, constant_phi, dead_exit, false);
            separate.eliminate_unreachable_code(&*isa).unwrap();
            separate.remove_constant_phis(&*isa).unwrap();
            pair.prune_unreachable_and_remove_constant_phis(&*isa)
                .unwrap();
            pair.verify(&*isa).unwrap();
            let layout = pair.func.layout.blocks().collect::<Vec<_>>();
            assert_eq!(layout.len(), 2);
            assert_eq!(
                pair.func.dfg.block_params(layout[1]).len(),
                usize::from(!constant_phi)
            );
            assert_eq!(
                pair.func.display().to_string(),
                separate.func.display().to_string()
            );
            assert_eq!(
                pair.domtree.cfg_postorder(),
                separate.domtree.cfg_postorder()
            );
        }
    }
}

#[test]
fn a_no_op_phi_sweep_still_verifies_the_pruned_function() {
    let isa = isa();
    let mut pair = function(&*isa, false, false, true);
    let before = function(&*isa, false, false, true)
        .eliminate_unreachable_code(&*isa)
        .unwrap_err();
    let after = pair
        .prune_unreachable_and_remove_constant_phis(&*isa)
        .unwrap_err();
    let (CodegenError::Verifier(before), CodegenError::Verifier(after)) = (before, after) else {
        panic!("expected the first verification to report the malformed return");
    };
    assert!(!after.is_empty());
    assert_eq!(before.to_string(), after.to_string());
}
