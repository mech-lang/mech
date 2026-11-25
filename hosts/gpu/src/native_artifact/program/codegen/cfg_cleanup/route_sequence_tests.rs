use super::super::FinalCleanupSchedule;
use super::*;
use crate::native_artifact::codegen::{
    backend as module_backend, compilation_inputs::CompilationInputs, options::OptimizationOptions,
    route_work::RouteWorkCollector,
};
use cranelift_codegen::ir::{InstructionData, types};
use cranelift_jit::JITModule;
use cranelift_module::Module;
use serde_json::Value;

fn module() -> JITModule {
    module_backend::jit_with_inputs(
        module_backend::CONTROL_FLAGS,
        &CompilationInputs::capture([]),
    )
    .unwrap()
}

fn identity_fixture(module: &JITModule, with_alias: bool) -> Context {
    let mut context = identity_routes::tests::same_join_fixture(module);
    if with_alias {
        let entry = context.func.layout.entry_block().unwrap();
        let source = context.func.dfg.block_params(entry)[0];
        let detached = context.func.dfg.make_block();
        let alias = context.func.dfg.append_block_param(detached, types::I64);
        context.func.dfg.detach_block_params(detached);
        context.func.dfg.change_to_alias(alias, source);
        let branch = context.func.layout.last_inst(entry).unwrap();
        let InstructionData::Brif { arg, .. } = &mut context.func.dfg.insts[branch] else {
            panic!("identity fixture lost its duplicate-successor branch");
        };
        *arg = alias;
    }
    context
}

fn controls(options: OptimizationOptions) -> CompilationControls {
    CompilationControls::with_inputs(options, &CompilationInputs::capture([]))
}

fn run(
    context: &mut Context,
    isa: &dyn TargetIsa,
    controls: &CompilationControls,
    cleanup: RouteCleanup,
    options: IdentityRouteOptions<'_>,
    paired: bool,
) -> Result<[Value; 2], NativeCompileError> {
    let (loops, identity) = if paired {
        let mut routes = RouteSequence::new(controls, context, isa, cleanup);
        let loops = routes.thread_loops()?;
        let identity = routes.forward_identity(options)?;
        (loops, identity)
    } else {
        let loops = RouteSequence::new(controls, context, isa, cleanup).thread_loops()?;
        let identity =
            RouteSequence::new(controls, context, isa, cleanup).forward_identity(options)?;
        (loops, identity)
    };
    Ok([
        serde_json::to_value(loops).unwrap(),
        serde_json::to_value(identity).unwrap(),
    ])
}

#[test]
fn paired_routes_match_fresh_sequences_across_cleanup_and_selection_modes() {
    let mut module = module();
    for cleanup in [
        RouteCleanup::Ordered,
        RouteCleanup::Structural {
            combined_verification: false,
        },
        RouteCleanup::Structural {
            combined_verification: true,
        },
    ] {
        for disabled in [
            None,
            Some(OptimizationPass::CfgCleanup),
            Some(OptimizationPass::LoopEdgeThreading),
            Some(OptimizationPass::IdentityRoutes),
        ] {
            let controls = controls(
                disabled.map_or_else(OptimizationOptions::default, OptimizationOptions::without),
            );
            for source in [
                identity_fixture(&module, true),
                threaded_edges::test_support::fixture(&module, "duplicate"),
            ] {
                let entry = [source.func.layout.entry_block().unwrap()];
                for options in [
                    IdentityRouteOptions::default(),
                    IdentityRouteOptions::new(Some(&entry), false),
                    IdentityRouteOptions::new(None, true),
                ] {
                    let mut fresh = Context::for_function(source.func.clone());
                    let mut paired = Context::for_function(source.func.clone());
                    let expect =
                        run(&mut fresh, module.isa(), &controls, cleanup, options, false).unwrap();
                    let actual =
                        run(&mut paired, module.isa(), &controls, cleanup, options, true).unwrap();
                    assert_eq!(actual, expect);
                    assert_eq!(paired.func.to_string(), fresh.func.to_string());
                    paired.verify(module.isa()).unwrap();
                }
            }
        }
    }
    let controls = controls(OptimizationOptions::default());
    let source = identity_fixture(&module, true);
    let mut fresh = Context::for_function(source.func.clone());
    let mut paired = source;
    let first = run(
        &mut fresh,
        module.isa(),
        &controls,
        RouteCleanup::Ordered,
        IdentityRouteOptions::default(),
        false,
    )
    .unwrap();
    let second = run(
        &mut paired,
        module.isa(),
        &controls,
        RouteCleanup::Ordered,
        IdentityRouteOptions::default(),
        true,
    )
    .unwrap();
    assert_eq!(first, second);
    assert_eq!(second[0]["threaded_edges"], 0);
    assert_eq!(second[1]["threaded_edges"], 2);
    let a = identity_routes::tests::define(&mut module, &mut fresh, "fresh_routes");
    let b = identity_routes::tests::define(&mut module, &mut paired, "paired_routes");
    assert_eq!(
        fresh.compiled_code().unwrap().buffer.data(),
        paired.compiled_code().unwrap().buffer.data()
    );
    for flag in [0, 1, u64::MAX] {
        let expected = if flag == 0 {
            (0, [41, 17])
        } else {
            (1, [17, 41])
        };
        for entry in [a, b] {
            let mut slots = [0; 2];
            let value = unsafe { entry(flag, 17, 41, 0, slots.as_mut_ptr()) };
            assert_eq!((value, slots), expected);
        }
    }
}

#[test]
fn failed_loop_cleanup_stops_the_following_identity_search() {
    let module = module();
    let work = RouteWorkCollector::new();
    let controls = controls(OptimizationOptions::default())
        .with_route_recorder(Some(work.register("first error")));
    let source = threaded_edges::test_support::fixture(&module, "normal");
    let mut invalid = Context::for_function(source.func.clone());
    invalid.func.signature.returns.clear();
    let error = run(
        &mut invalid,
        module.isa(),
        &controls,
        RouteCleanup::Ordered,
        IdentityRouteOptions::default(),
        true,
    )
    .unwrap_err();
    assert!(!error.to_string().is_empty());
    drop(controls);
    let totals = work.json()["totals"].clone();
    assert_eq!(totals["loop_searches"], 1);
    assert!(totals["loop_edges_rewritten"].as_u64().unwrap() > 0);
    assert_eq!(totals["identity_searches"], 0);
}

#[test]
fn final_schedule_preserves_exact_legacy_os_modes() {
    use FinalCleanupSchedule::*;
    use std::ffi::OsStr;
    assert_eq!(FinalCleanupSchedule::from_legacy(None), Disabled);
    for (input, expected) in [
        ("ordered", Ordered),
        ("routes", Routes),
        ("batched-routes", BatchedRoutes),
        ("lean-routes", LeanRoutes),
        ("", Full),
        ("routes ", Full),
        ("ORDERED", Full),
        ("unexpected", Full),
    ] {
        assert_eq!(
            FinalCleanupSchedule::from_legacy(Some(OsStr::new(input))),
            expected
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        assert_eq!(
            FinalCleanupSchedule::from_legacy(Some(OsStr::from_bytes(b"\xff"))),
            Full
        );
    }
}

#[cfg(feature = "construction-census")]
#[test]
fn route_sequence_reuses_only_zero_edge_graphs() {
    use cranelift_codegen::construction_census::snapshot;
    let module = module();
    let controls = controls(OptimizationOptions::default());
    for (fixture, expected_saved) in [
        (identity_fixture(&module, true), 1),
        (
            threaded_edges::test_support::fixture(&module, "duplicate"),
            0,
        ),
    ] {
        for cleanup in [
            RouteCleanup::Ordered,
            RouteCleanup::Structural {
                combined_verification: true,
            },
            RouteCleanup::Structural {
                combined_verification: false,
            },
        ] {
            let mut fresh = Context::for_function(fixture.func.clone());
            let mut paired = Context::for_function(fixture.func.clone());
            let run_case = |context: &mut Context, paired| {
                let before = snapshot();
                let report = run(
                    context,
                    module.isa(),
                    &controls,
                    cleanup,
                    IdentityRouteOptions::default(),
                    paired,
                )
                .unwrap();
                let after = snapshot();
                (
                    report,
                    [
                        after.cfg_builds - before.cfg_builds,
                        after.dominator_builds - before.dominator_builds,
                        after.full_verifications - before.full_verifications,
                    ],
                )
            };
            let (fresh_report, fresh_counts) = run_case(&mut fresh, false);
            let (paired_report, paired_counts) = run_case(&mut paired, true);
            assert_eq!(fresh_report, paired_report);
            assert_eq!(paired.func.to_string(), fresh.func.to_string());
            assert_eq!(fresh_counts[0] - paired_counts[0], expected_saved);
            assert_eq!(fresh_counts[1] - paired_counts[1], expected_saved);
            assert_eq!(fresh_counts[2], paired_counts[2]);
            if expected_saved == 1 {
                assert_eq!(paired_report[0]["threaded_edges"], 0);
                assert!(paired_report[1]["threaded_edges"].as_u64().unwrap() > 0);
            } else {
                assert!(paired_report[0]["threaded_edges"].as_u64().unwrap() > 0);
            }
        }
    }

    let work = RouteWorkCollector::new();
    let controls = self::controls(OptimizationOptions::default())
        .with_route_recorder(Some(work.register("one reuse")));
    let mut repeated = identity_fixture(&module, true);
    {
        let mut sequence = RouteSequence::new(
            &controls,
            &mut repeated,
            module.isa(),
            RouteCleanup::Ordered,
        );
        assert_eq!(sequence.thread_loops().unwrap().threaded_edges, 0);
        sequence
            .forward_identity(IdentityRouteOptions::default())
            .unwrap();
        sequence
            .forward_identity(IdentityRouteOptions::default())
            .unwrap();
    }
    drop(controls);
    let data = work.json();
    assert_eq!(data["totals"]["cfg_requests"], 3);
    assert_eq!(data["totals"]["cfg_builds"], 2);
    assert_eq!(data["totals"]["dom_builds"], 2);
    assert_eq!(data["totals"]["graph_pair_reuses"], 1);
}
