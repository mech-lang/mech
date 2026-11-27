//! Explicit compile-time inventory; this diagnostic never grants runtime credit.
use super::*;

#[test]
#[ignore = "explicit source-plan diagnostic; requires an output directory"]
fn diagnose_numeric_child_composition() {
    use crate::native_artifact::program::prepare::{
        numeric_maps, numeric_phase::matrix_maps, readonly_numeric,
    };
    use mech_engine::native_artifact::{NativeControlLoop, NativeIterationAction};
    use serde_json::json;
    let folder = std::path::PathBuf::from(
        std::env::var_os("MECH_NUMERIC_COMPOSITION_DIAGNOSTIC")
            .expect("explicit diagnostic output directory"),
    );
    let source = include_str!("../../../../../tests/fixtures/polybench/deriche.mec");
    let artifact = artifacts(source).into_iter().next().unwrap();
    let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
    let m = jit.descriptor().metadata();
    let blocks = m.plan.blocks().iter().enumerate().map(|(site, block)| {
        let action = match block.action() {
            NativeIterationAction::LibraryCall { operands, output, .. } => json!({
                "kind":"LibraryCall",
                "primitive":format!("{:?}",m.calls[site].as_ref().unwrap().template().primitive()),
                "operands":operands.iter().map(|v|v.get()).collect::<Vec<_>>(),
                "output":output.get(),
            }),
            action => json!(format!("{action:?}")),
        };
        json!({"site":site,"origin":format!("{:?}",block.origin()),"iteration":block.iteration(),
            "action":action,"terminator":format!("{:?}",block.terminator()),
            "frame_action":m.buffer_frames.action(site).map(|v|format!("{v:?}"))})
    }).collect::<Vec<_>>();
    let tails = m.plan.iterations().iter().enumerate().filter_map(|(id, control)| {
        let NativeControlLoop::Tail(tail)=control else{return None};
        let result=readonly_numeric::derive(&m.plan,tail,&m.buffer_frames,&m.scalar_aggregates,&m.calls,&m.visits);
        let complete=readonly_numeric::derive_complete_for_diagnostic(&m.plan,tail,&m.buffer_frames,&m.scalar_aggregates,&m.calls,&m.visits);
        Some(json!({"iteration":id,"header":tail.header(),"parent":tail.parent(),"carrier":tail.carrier().get(),
            "proof_rejection":result.as_ref().err(),"complete_rejection":complete.as_ref().err(),
            "selected":m.closed_tails.regions.iter().any(|r|r.iteration as usize==id&&r.numeric.is_some()),
            "proof":result.as_ref().ok().map(|p|json!({"arrays":p.arrays,"maps":p.map_banks,"native_sites":p.sites,
                "phase":p.phase,"exits":p.exits,"scalar_inputs":p.scalar_inputs,"captures":p.scalar_captures}))}))
    }).collect::<Vec<_>>();
    let collections=m.plan.collections().iter().enumerate().map(|(id,c)|json!({
        "id":id,"kind":format!("{:?}",c.kind()),"parent":c.parent(),"generators":c.generators(),
        "begin":c.begin(),"finish":c.finish(),"output":c.output().get(),"source":format!("{c:?}")
    })).collect::<Vec<_>>();
    let values=m.plan.values().iter().enumerate().map(|(id,v)|json!({
        "id":id,"schema":format!("{:?}",v.schema()),"body":format!("{:?}",v.definition().body()),
        "scalar":v.scalar_kind().map(|k|format!("{k:?}")),"scope":format!("{:?}",m.plan.value_scopes()[id])
    })).collect::<Vec<_>>();
    let report = json!({"scope":"optimization-set Deriche compiler inventory only; no runtime/performance measurement",
        "blocks":blocks,"values":values,"tails":tails,"collections":collections,
        "iterations":m.plan.iterations().iter().map(|v|format!("{v:?}")).collect::<Vec<_>>(),
        "numeric_maps":numeric_maps::Layout::derive(&m.plan).unwrap(),
        "phase_maps":matrix_maps::inventory(&m.plan).unwrap(),
        "selected_numeric_regions":m.closed_tails.regions.iter().filter(|r|r.numeric.is_some()).map(|r|r.iteration).collect::<Vec<_>>()});
    std::fs::write(
        folder.join("deriche-plan.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!(
        "blocks={} collections={} tails={}",
        m.plan.blocks().len(),
        m.plan.collections().len(),
        report["tails"].as_array().unwrap().len()
    );
}
