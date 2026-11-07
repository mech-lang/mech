//! Current-visit Boolean implications for the numeric must-current proof.
//! All producers establish zero/one; declared Bool, raw words, unknown joins,
//! carriers, tags and cycles provide no implication. No execution is removed.
use super::*;
use mech_engine::native_artifact::{
    NativeAggregateWord as Word, NativeScalarAggregateAction as Aggregate,
    NativeScalarKind as Kind, NativeScalarOp as Op,
};

type Edges = Vec<(usize, (usize, usize))>;

pub(super) struct Related {
    pub edges: Edges,
    pub allowed: [BTreeMap<usize, usize>; 2],
}

fn boolean_key(facts: &NativeProgramFacts<'_>, key: Key) -> bool {
    let Some(ty) = facts.plan().values().get(key.0 as usize) else {
        return false;
    };
    match key.1 {
        None => ty.scalar_kind() == Some(Kind::Bool),
        Some(field) => {
            let Some((_, words)) = facts.scalar_aggregates().slot_at(key.0 as usize) else {
                return false;
            };
            match ty.definition().body() {
                SchemaBody::Tuple(fields) if fields.len() == words => {
                    fields.get(field) == Some(&SchemaBody::Bool)
                }
                SchemaBody::Record(fields) if fields.len() == words => fields
                    .get(field)
                    .is_some_and(|f| f.schema == SchemaBody::Bool),
                _ => false,
            }
        }
    }
}

fn copied(facts: &NativeProgramFacts<'_>, site: usize, key: Key) -> Option<Key> {
    let block = facts.plan().blocks().get(site)?;
    if let Action::Scalar(Scalar::Copy { source, output }) = block.action()
        && output.get() == key.0
    {
        return Some((source.get(), key.1));
    }
    match facts.scalar_aggregates().action(site)? {
        Aggregate::Copy { source, output } if output.get() == key.0 => Some((source.get(), key.1)),
        Aggregate::Project {
            source,
            offset,
            guards,
            output,
        } if output.get() == key.0 && guards.is_empty() => {
            Some((source.get(), Some(offset.checked_add(key.1.unwrap_or(0))?)))
        }
        Aggregate::Observe {
            source,
            offset,
            ordinal: None,
            guards,
            output,
        } if output.get() == key.0 && key.1.is_none() && guards.is_empty() => {
            Some((source.get(), Some(*offset)))
        }
        _ => None,
    }
}

fn constant(facts: &NativeProgramFacts<'_>, key: Key) -> bool {
    if key.1.is_none() {
        facts.plan().constants().iter().any(|c| {
            c.value().get() == key.0 && matches!(c.constant().data(), mech_core::ValueData::Bool(_))
        })
    } else {
        facts
            .scalar_aggregates()
            .constants()
            .iter()
            .any(|(v, words)| {
                v.get() == key.0
                    && key
                        .1
                        .and_then(|at| words.get(at))
                        .is_some_and(|&word| word <= 1)
            })
    }
}

// A bounded acyclic proof checks EVERY producer, including flat tuple fields.
// Refusing a cycle is conservative; neither a declared type nor sample bits
// seed an induction about canonical representation.
fn canonical(facts: &NativeProgramFacts<'_>, key: Key, depth: usize, fuel: &mut usize) -> bool {
    if depth == 0 || *fuel == 0 || !boolean_key(facts, key) {
        return false;
    }
    *fuel -= 1;
    if constant(facts, key) {
        return true;
    }
    let Ok(definitions) = facts.definition_sites(key.0) else {
        return false;
    };
    !definitions.is_empty()
        && definitions.iter().all(|definition| {
            let Some(site) = definition.site().map(|site| site as usize) else {
                return false;
            };
            if let Some(source) = copied(facts, site, key) {
                return canonical(facts, source, depth - 1, fuel);
            }
            if let Some(Aggregate::Pack { output, words }) = facts.scalar_aggregates().action(site)
                && output.get() == key.0
                && let Some(word) = key.1.and_then(|field| words.get(field))
            {
                return match word {
                    Word::Scalar(value) => canonical(facts, (value.get(), None), depth - 1, fuel),
                    Word::Field(value, field) => {
                        canonical(facts, (value.get(), Some(*field)), depth - 1, fuel)
                    }
                    Word::Constant(word) => *word <= 1,
                };
            }
            let Action::Scalar(Scalar::Primitive {
                op,
                operands,
                output,
            }) = facts.plan().blocks()[site].action()
            else {
                return false;
            };
            if output.get() != key.0 || key.1.is_some() {
                return false;
            }
            match op {
                Op::BoolAnd | Op::BoolOr | Op::BoolXor | Op::BoolNot => operands
                    .iter()
                    .all(|v| canonical(facts, (v.get(), None), depth - 1, fuel)),
                Op::U8Eq
                | Op::U8Ne
                | Op::U8Lt
                | Op::U8Le
                | Op::U8Gt
                | Op::U8Ge
                | Op::U16Eq
                | Op::U16Ne
                | Op::U16Lt
                | Op::U16Le
                | Op::U16Gt
                | Op::U16Ge
                | Op::U32Eq
                | Op::U32Ne
                | Op::U32Lt
                | Op::U32Le
                | Op::U32Gt
                | Op::U32Ge
                | Op::U64Eq
                | Op::U64Ne
                | Op::U64Lt
                | Op::U64Le
                | Op::U64Gt
                | Op::U64Ge
                | Op::I64Eq
                | Op::I64Ne
                | Op::I64Lt
                | Op::I64Le
                | Op::I64Gt
                | Op::I64Ge
                | Op::F64Eq
                | Op::F64Ne
                | Op::F64Lt
                | Op::F64Le
                | Op::F64Gt
                | Op::F64Ge
                | Op::BoolEq
                | Op::BoolNe => true,
                _ => false,
            }
        })
}

impl Traversal {
    // Multiple immutable definitions must collectively dominate THIS visit's
    // reader. Stop at every producer; root backedges are already excluded.
    fn current_key(
        &self,
        facts: &NativeProgramFacts<'_>,
        mut key: Key,
        mut reader: usize,
    ) -> Option<Key> {
        for _ in 0..64 {
            if !boolean_key(facts, key) {
                return None;
            }
            if constant(facts, key) {
                return Some(key);
            }
            let iteration = facts.plan().blocks()[self.header].iteration();
            if facts.plan().value_scopes()[key.0 as usize] != (Scope::Immutable { iteration }) {
                return None;
            }
            let definitions = facts.definition_sites(key.0).ok()?;
            if definitions.is_empty() {
                return None;
            }
            let sites = definitions
                .iter()
                .map(|d| {
                    let site = d.site()? as usize;
                    (d.iteration() == iteration && self.next.contains_key(&site)).then_some(site)
                })
                .collect::<Option<BTreeSet<_>>>()?;
            let mut pending = vec![self.header];
            let mut seen = BTreeSet::new();
            while let Some(site) = pending.pop() {
                if sites.contains(&site) || !seen.insert(site) {
                    continue;
                }
                if site == reader {
                    return None;
                }
                pending.extend(&self.next[&site]);
            }
            if sites.len() == 1 {
                let site = *sites.first()?;
                if let Some(source) = copied(facts, site, key) {
                    key = source;
                    reader = site;
                    continue;
                }
            }
            return Some(key);
        }
        None
    }
}

fn imply(
    traversal: &Traversal,
    facts: &NativeProgramFacts<'_>,
    key: Key,
    truth: bool,
    out: &mut BTreeMap<Key, bool>,
    depth: usize,
    fuel: &mut usize,
) -> Option<()> {
    if depth == 0 || *fuel == 0 {
        return None;
    }
    *fuel -= 1;
    if let Some(old) = out.insert(key, truth) {
        return (old == truth).then_some(());
    }
    if key.1.is_some() {
        return Some(());
    }
    let definitions = facts.definition_sites(key.0).ok()?;
    let [definition] = definitions else {
        return Some(());
    };
    let Some(site) = definition.site().map(|site| site as usize) else {
        return Some(());
    };
    let Action::Scalar(Scalar::Primitive {
        op,
        operands,
        output,
    }) = facts.plan().blocks()[site].action()
    else {
        return Some(());
    };
    if output.get() != key.0 {
        return None;
    }
    if matches!(
        (op, truth),
        (Op::BoolAnd, true) | (Op::BoolOr, false) | (Op::BoolNot, _)
    ) {
        for operand in operands {
            let source = traversal.current_key(facts, (operand.get(), None), site)?;
            imply(
                traversal,
                facts,
                source,
                if *op == Op::BoolNot { !truth } else { truth },
                out,
                depth - 1,
                fuel,
            )?;
        }
    }
    Some(())
}

pub(super) fn derive(traversal: &Traversal, facts: &NativeProgramFacts<'_>) -> Vec<Related> {
    let mut tests = BTreeMap::<Key, Edges>::new();
    for &site in &traversal.order {
        let operands = match facts.plan().blocks()[site].terminator() {
            Term::Scalar(Branch::Branch { condition, .. }) => vec![condition],
            Term::Scalar(Branch::LiteralBranch { left, right, .. }) => vec![left, right],
            _ => continue,
        };
        for value in operands {
            if let Some((no, yes)) = facts.boolean_branch_edges(site as u32, value)
                && let Some(key) = traversal.current_key(facts, (value.get(), None), site)
            {
                tests
                    .entry(key)
                    .or_default()
                    .push((site, (no as usize, yes as usize)));
            }
        }
    }
    let mut result = Vec::new();
    for (&key, edges) in &tests {
        if !canonical(facts, key, 64, &mut 512) {
            continue;
        }
        let mut allowed = [BTreeMap::new(), BTreeMap::new()];
        let mut extra = false;
        for (at, truth) in [false, true].into_iter().enumerate() {
            let mut implied = BTreeMap::new();
            if imply(traversal, facts, key, truth, &mut implied, 32, &mut 256).is_none() {
                continue;
            }
            for (other, known) in implied {
                let Some(tests) = tests.get(&other) else {
                    continue;
                };
                for &(site, (no, yes)) in tests {
                    allowed[at].insert(site, if known { yes } else { no });
                    extra |= !edges.iter().any(|(original, _)| *original == site);
                }
            }
        }
        if extra {
            result.push(Related {
                edges: edges.clone(),
                allowed,
            });
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };

    #[test]
    fn canonical_relation_requires_every_tuple_field_producer_and_refuses_raw_inputs() {
        let source = "choice := (selector<bool>? | true => unknown<bool> | false => true)\npair := (3u64,choice)\n(pair.2? | true => 1u64 | false => 0u64)\n";
        for (text, expected) in [
            (source.to_owned(), false),
            (source.replace("unknown<bool>", "(seed<u64> == 7u64)"), true),
        ] {
            let parsed = parse_canonical_document(
                TextSnapshot::new(DocumentId(0x424f4f4c), Revision(1), &text).unwrap(),
                ParseConfig::default(),
            );
            assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
            let artifact = mech_engine::CanonicalSourceFrontend
                .compile_document_with_catalog(
                    &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                    mech_stdlib::source_catalog(),
                )
                .unwrap()
                .compile_artifact()
                .unwrap();
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                NativeProgramLimitMode::NoLimits.compilation(),
            )
            .unwrap();
            let facts = jit.descriptor().metadata().plan.facts();
            let mut fields = 0;
            for index in 0..facts.plan().values().len() {
                let key = (index as u32, Some(1));
                if boolean_key(&facts, key) {
                    fields += 1;
                    assert_eq!(canonical(&facts, key, 64, &mut 512), expected);
                    assert!(!canonical(&facts, key, 64, &mut 0));
                }
            }
            assert!(
                fields > 0,
                "exercise a private field whose producers meet at the conditional"
            );
            for index in 0..facts.plan().transport_input_count() {
                let key = (facts.plan().transport_input_value(index).get(), None);
                if boolean_key(&facts, key) {
                    assert!(!canonical(&facts, key, 64, &mut 512));
                }
            }
        }
    }
}
