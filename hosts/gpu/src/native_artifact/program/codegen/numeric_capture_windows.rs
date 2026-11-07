//! Gate cloned immutable affine children with synchronous checked handoffs.
use super::*;
use crate::native_artifact::program::codegen::traversal_facts::CapturedWindow;
use crate::native_artifact::program::prepare::numeric_phase::captured_windows as coordinates;
use cranelift_codegen::ir::types;
use mech_engine::native_artifact::NativeProgramFacts;
use std::collections::BTreeSet;

pub(super) fn derive(
    code: CodePlan<'_>,
    proof: &Proof,
    shadow: usize,
    b: &mut FunctionBuilder<'_>,
    packs: &[super::super::numeric_read_packs::Pack],
    existing: &[Version],
    child_funding: bool,
) -> Vec<Version> {
    let CodePlan::Program(program) = code else {
        return Vec::new();
    };
    let mut versions = Vec::new();
    for (id, control) in program.plan().iterations().iter().enumerate() {
        let NativeControlLoop::Tail(tail) = control else {
            continue;
        };
        if tail.parent().is_none()
            || !proof.sites.contains(&(tail.initialize() as usize))
            || existing.iter().any(|v| v.header == tail.header() as usize)
        {
            continue;
        }
        if !matches!(code.blocks()[tail.initialize() as usize].terminator(),
            Term::TailHeader { body, .. } | Term::Scalar(Branch::Jump(body)) if body == tail.header())
        {
            continue;
        }
        let candidate = (|| {
            let scope = scope(code, proof, id as u32, tail)?;
            let reads = packs
                .iter()
                .filter(|pack| {
                    scope.sites.contains(&pack.first)
                        && pack.source_window
                        && pack.offset == 1
                        && !proof.arrays[pack.array].writable
                        && !proof.arrays[pack.array].appends
                })
                .map(|pack| coordinates::Read {
                    site: pack.first,
                    base: pack.base.get(),
                    array: pack.array,
                    width: pack.width,
                })
                .collect::<Vec<_>>();
            let current = coordinates::derive(
                program.plan(),
                &program.metadata().buffer_frames,
                &program.metadata().scalar_aggregates,
                tail,
                &reads,
            )?;
            let fields = proof.fields(tail.carrier().get())?;
            let at = |ordinal: usize| {
                fields
                    .get(ordinal)
                    .copied()
                    .flatten()
                    .map(|word| shadow + word)
            };
            let captures = current
                .bases
                .into_iter()
                .map(|(array, base)| {
                    Some(CapturedWindow {
                        array,
                        base: at(base)?,
                        induction: at(current.induction)?,
                        limit: at(current.limit)?,
                        width: current.step,
                    })
                })
                .collect::<Option<Vec<_>>>()?;
            let facts = super::super::numeric_traversal::captured_version(
                code, proof, shadow, &scope, &captures,
            )?;
            if facts.successors.len() < 2 {
                return None;
            }
            Some((scope, captures, facts))
        })();
        if let Some((scope, captures, facts)) = candidate {
            let funding = child_funding
                .then(|| {
                    super::super::numeric_child_funding::Funding::derive(
                        proof,
                        scope.header,
                        &captures,
                        b,
                    )
                })
                .flatten();
            versions.push(Version {
                initialize: tail.initialize() as usize,
                header: scope.header,
                blocks: scope
                    .sites
                    .iter()
                    .map(|&s| (s as u32, b.create_block()))
                    .collect(),
                sites: scope.sites,
                windows: Vec::new(),
                captures,
                facts,
                prepaid: None,
                reduction: None,
                funding,
            });
        }
    }
    versions
}

fn scope(
    code: CodePlan<'_>,
    proof: &Proof,
    owner: u32,
    tail: &mech_engine::native_artifact::NativeTailLoop,
) -> Option<Scope> {
    let mut scope = Scope {
        header: tail.header() as usize,
        latch: tail.latch() as usize,
        exit: tail.exit(),
        sites: Vec::new(),
        outside: BTreeSet::new(),
    };
    let mut pending = vec![scope.header];
    let mut seen = BTreeSet::new();
    while let Some(site) = pending.pop() {
        if !seen.insert(site) {
            continue;
        }
        if !proof.sites.contains(&site)
            || code.blocks()[site].iteration() != Some(owner)
            || matches!(
                proof.action(site),
                Some(
                    Operation::Fallback
                        | Operation::Write { .. }
                        | Operation::Append { .. }
                        | Operation::AppendLeBytes { .. }
                )
            )
        {
            scope.outside.insert(site as u32);
            continue;
        }
        scope.sites.push(site);
        if site != scope.latch {
            pending.extend(
                NativeProgramFacts::control_successors(code.blocks()[site].terminator())
                    .into_iter()
                    .flatten()
                    .filter(|&next| next != scope.exit)
                    .map(|site| site as usize),
            );
        }
    }
    scope.sites.sort_unstable();
    scope.order(code)?;
    Some(scope)
}

pub(super) fn gate(
    version: &Version,
    b: &mut FunctionBuilder<'_>,
    words: &Words,
    shadow: usize,
    proof: &Proof,
    facts: &Facts,
    state: usize,
    checked: Block,
) {
    let mut enough = b.ins().iconst(types::I8, 1);
    for window in &version.captures {
        let start = words.read(b, window.induction);
        let limit = words.read(b, window.limit);
        let ordered = if facts.check(
            version.initialize,
            Check::Le(Operand::Word(window.induction), Operand::Word(window.limit)),
        ) {
            b.ins().iconst(types::I8, 1)
        } else {
            b.ins().icmp(IntCC::UnsignedLessThanOrEqual, start, limit)
        };
        let long = if facts.check(
            version.initialize,
            Check::Le(Operand::Constant(window.width), Operand::Word(window.limit)),
        ) {
            b.ins().iconst(types::I8, 1)
        } else {
            b.ins().icmp_imm(
                IntCC::UnsignedGreaterThanOrEqual,
                limit,
                window.width as i64,
            )
        };
        let safe = if facts.check(
            version.initialize,
            Check::Addition {
                word: window.limit,
                extra: window.width,
            },
        ) {
            b.ins().iconst(types::I8, 1)
        } else {
            b.ins().icmp_imm(
                IntCC::UnsignedLessThanOrEqual,
                limit,
                (u64::MAX - window.width) as i64,
            )
        };
        let view = shadow + proof.view(window.array);
        let length = words.read(b, view + 1);
        let base = words.read(b, window.base);
        let inside = if facts.check(
            version.initialize,
            Check::Le(Operand::Word(window.base), Operand::Length(window.array)),
        ) {
            b.ins().iconst(types::I8, 1)
        } else {
            b.ins().icmp(IntCC::UnsignedLessThanOrEqual, base, length)
        };
        let available = b.ins().isub(length, base);
        let fits = if facts.check(
            version.initialize,
            Check::Fits {
                array: window.array,
                base: Some(window.base),
                count: window.limit,
                extra: 0,
            },
        ) {
            b.ins().iconst(types::I8, 1)
        } else {
            b.ins()
                .icmp(IntCC::UnsignedLessThanOrEqual, limit, available)
        };
        let rows = words.read(b, view + 2);
        let columns = words.read(b, view + 3);
        let row = b.ins().icmp_imm(IntCC::Equal, rows, 1);
        let column = b.ins().icmp_imm(IntCC::Equal, columns, 1);
        let flat = b.ins().bor(row, column);
        for valid in [ordered, long, safe, inside, fits, flat] {
            enough = b.ins().band(enough, valid);
        }
    }
    if let Some(funding) = &version.funding {
        funding.gate(b, words, state, shadow, enough, checked);
        return;
    }
    b.ins().brif(
        enough,
        version.blocks[&(version.header as u32)],
        &[],
        checked,
        &[],
    );
}
