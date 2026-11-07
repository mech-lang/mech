//! Immutable view geometry needs no affine carrier or loop-trip bound.
use super::*;
use cranelift_codegen::ir::types;
use std::collections::BTreeSet;

pub(super) fn derive(
    code: CodePlan<'_>,
    proof: &Proof,
    shadow: usize,
    b: &mut FunctionBuilder<'_>,
    packs: &[super::super::numeric_read_packs::Pack],
    existing: &[Version],
) -> Vec<Version> {
    let CodePlan::Program(program) = code else {
        return Vec::new();
    };
    program
        .plan()
        .iterations()
        .iter()
        .enumerate()
        .filter_map(|(id, control)| {
            let NativeControlLoop::Tail(tail) = control else {
                return None;
            };
            if tail.parent().is_none()
                || !proof.sites.contains(&(tail.initialize() as usize))
                || program.metadata().scalar_aggregates.slot(tail.carrier()).is_none()
                || !matches!(program.plan().values()[tail.carrier().get() as usize].definition().body(), mech_core::SchemaBody::Enum { .. })
                || existing.iter().any(|v| v.header == tail.header() as usize)
                || !matches!(code.blocks()[tail.initialize() as usize].terminator(),
                    Term::TailHeader { body, .. } | Term::Scalar(Branch::Jump(body)) if body == tail.header())
            {
                return None;
            }
            let scope = captured::scope(code, proof, id as u32, tail)?;
            let sites = scope.sites.iter().copied().collect::<BTreeSet<_>>();
            let geometry = packs
                .iter()
                .filter(|pack| {
                    pack.source_window
                        && sites.contains(&pack.first)
                        && !proof.arrays[pack.array].writable
                        && !proof.arrays[pack.array].appends
                })
                .map(|pack| pack.array)
                .collect::<BTreeSet<_>>();
            if geometry.is_empty() {
                return None;
            }
            // The existing numeric proof binds each view to the same immutable
            // root across every child alternative and backedge. This gate says
            // nothing about indices, arithmetic, tags, limits or visit counts.
            // Affine/value analysis may lawfully decline a large current
            // visit. Immutable geometry does not depend on that analysis or
            // on acyclicity: unsupported substitutions remain checked.
            let mut facts = super::super::numeric_traversal::local(
                code, proof, shadow, &scope, &[],
            )
            .unwrap_or_default();
            facts.flat_arrays.extend(geometry.iter().copied());
            Some(Version {
                initialize: tail.initialize() as usize,
                header: scope.header,
                blocks: scope.sites.iter().map(|&s| (s as u32, b.create_block())).collect(),
                sites: scope.sites,
                windows: Vec::new(),
                captures: Vec::new(),
                geometry: geometry.into_iter().collect(),
                addresses: Vec::new(),
                facts,
                prepaid: None,
                reduction: None,
                funding: None,
            })
        })
        .collect()
}

pub(super) fn gate(
    version: &Version,
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    shadow: usize,
    proof: &Proof,
    checked: Block,
) {
    let mut enough = b.ins().iconst(types::I8, 1);
    for &array in &version.geometry {
        let view = shadow + proof.view(array);
        let rows = words.read(b, view + 2);
        let columns = words.read(b, view + 3);
        let row = b.ins().icmp_imm(IntCC::Equal, rows, 1);
        let column = b.ins().icmp_imm(IntCC::Equal, columns, 1);
        let flat = b.ins().bor(row, column);
        enough = b.ins().band(enough, flat);
    }
    // Refusal is an ordinary checked child, never a new source rejection.
    b.ins().brif(
        enough,
        version.blocks[&(version.header as u32)],
        &[],
        checked,
        &[],
    );
}
