//! Exercise real source producers rather than inferring facts from Bool types.
use super::*;
use crate::native_artifact::codegen::{
    common::canonical_booleans::direct_literal,
    control::plan::CodePlan,
    options::{OptimizationOptions, OptimizationPass},
};
use crate::native_artifact::program::prepare::PreparedNativeProgram;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeMatchAction as Scalar, NativeScalarKind,
};

#[test]
fn canonical_boolean_literal_proofs_cover_copies_joins_and_refuse_raw_or_mixed_inputs() {
    for true_first in [false, true] {
        for mixed in [false, true] {
            let mut source = boolean_edges::alternate_source(true_first, true);
            if mixed {
                let old = "false => at < length - 8u64";
                assert_eq!(source.matches(old).count(), 1);
                source = source.replace(old, "false => gate");
            }
            for artifact in artifacts(&source) {
                let catalog = mech_stdlib::source_catalog();
                let native = JitProgramArtifact::compile_with_limits(
                    &artifact,
                    &catalog,
                    NativeProgramLimitMode::NoLimits.compilation(),
                )
                .unwrap();
                let prepared = PreparedNativeProgram::prepare(
                    &artifact,
                    &catalog,
                    native.descriptor().metadata().plan.clone(),
                )
                .unwrap();
                let code = CodePlan::Program(&prepared);
                let facts = prepared.plan().facts();
                let constant = (0..code.constant_count())
                    .find_map(|index| {
                        let (value, word) = code.constant(index).ok()?;
                        (code.kind(value) == Some(NativeScalarKind::Bool) && word == Some(1))
                            .then_some(value)
                    })
                    .unwrap();
                let enabled = OptimizationOptions::default();
                let disabled =
                    OptimizationOptions::without(OptimizationPass::CanonicalBooleanBranches);
                let mut raw_inputs = 0;
                for index in 0..code.input_count() {
                    let input = code.input(index);
                    if code.kind(input) == Some(NativeScalarKind::Bool) {
                        raw_inputs += 1;
                        assert!(direct_literal(code, enabled, input, constant).is_none());
                    }
                }
                assert!(raw_inputs > 0);
                let (mut copies, mut joins, mut rejected_joins) = (0, 0, 0);
                for block in code.blocks() {
                    let (Action::Scalar(Scalar::Copy { output, .. })
                    | Action::Scalar(Scalar::Primitive { output, .. })) = block.action()
                    else {
                        continue;
                    };
                    if code.kind(*output) != Some(NativeScalarKind::Bool) {
                        continue;
                    }
                    let result = direct_literal(code, enabled, *output, constant);
                    assert!(direct_literal(code, disabled, *output, constant).is_none());
                    if result.is_some()
                        && matches!(block.action(), Action::Scalar(Scalar::Copy { .. }))
                    {
                        copies += 1;
                    }
                    if facts.definition_sites(output.get()).unwrap().len() > 1 {
                        if result.is_some() {
                            joins += 1;
                        } else {
                            rejected_joins += 1;
                        }
                    }
                }
                if mixed {
                    assert!(
                        rejected_joins > 0,
                        "one unknown reaching producer must decline"
                    );
                    assert_eq!(
                        joins, 0,
                        "a raw reaching producer must keep its literal test"
                    );
                } else {
                    assert!(copies > 0, "canonical copies must survive");
                    assert!(joins > 0, "all-proved reaching producers must select");
                }
                println!(
                    "canonical-boolean-proof true_first={true_first} mixed={mixed} raw_inputs={raw_inputs} copies={copies} joins={joins} rejected_joins={rejected_joins}"
                );
            }
        }
    }
}
